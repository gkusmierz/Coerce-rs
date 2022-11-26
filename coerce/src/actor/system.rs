use crate::actor::scheduler::{start_actor, ActorScheduler, ActorType, GetActor, RegisterActor};
use crate::actor::{new_actor_id, Actor, ActorId, ActorRefErr, IntoActorId, LocalActorRef};
use rand::RngCore;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use uuid::Uuid;

#[cfg(feature = "remote")]
use crate::remote::system::RemoteActorSystem;

#[cfg(feature = "persistence")]
use crate::persistent::{journal::provider::StorageProvider, Persistence};

lazy_static! {
    static ref CURRENT_SYSTEM: ActorSystem = ActorSystem::new();
}

#[derive(Clone)]
pub struct ActorSystem {
    core: Arc<ActorSystemCore>,
}

#[derive(Clone)]
pub struct ActorSystemCore {
    system_id: Uuid,
    scheduler: LocalActorRef<ActorScheduler>,
    is_terminated: Arc<AtomicBool>,
    context_counter: Arc<AtomicU64>,

    #[cfg(feature = "persistence")]
    persistence: Option<Arc<Persistence>>,

    #[cfg(feature = "remote")]
    remote: Option<RemoteActorSystem>,
}

impl Default for ActorSystem {
    fn default() -> Self {
        let system_id = Uuid::new_v4();
        ActorSystem {
            core: Arc::new(ActorSystemCore {
                system_id,
                scheduler: ActorScheduler::new(system_id),

                is_terminated: Arc::new(AtomicBool::new(false)),
                context_counter: Arc::new(AtomicU64::new(1)),

                #[cfg(feature = "persistence")]
                persistence: None,

                #[cfg(feature = "remote")]
                remote: None,
            }),
        }
    }
}

impl ActorSystem {
    pub fn new() -> ActorSystem {
        Self::default()
    }

    #[cfg(feature = "persistence")]
    pub fn new_persistent<S: StorageProvider>(storage_provider: S) -> ActorSystem {
        ActorSystem::new().to_persistent(Persistence::from(storage_provider))
    }

    pub fn system_id(&self) -> &Uuid {
        &self.core.system_id
    }

    pub fn scheduler(&self) -> &LocalActorRef<ActorScheduler> {
        &self.core.scheduler
    }

    pub fn global_system() -> ActorSystem {
        CURRENT_SYSTEM.clone()
    }

    pub fn next_context_id(&self) -> u64 {
        self.core.context_counter.fetch_add(1, Relaxed)
    }

    pub async fn new_tracked_actor<A: Actor>(
        &self,
        actor: A,
    ) -> Result<LocalActorRef<A>, ActorRefErr> {
        let id = new_actor_id();
        self.new_actor(id, actor, ActorType::Tracked).await
    }

    pub async fn new_anon_actor<A: Actor>(
        &self,
        actor: A,
    ) -> Result<LocalActorRef<A>, ActorRefErr> {
        let id = new_actor_id();
        self.new_actor(id, actor, ActorType::Anonymous).await
    }

    pub async fn new_actor<I: IntoActorId, A: Actor>(
        &self,
        id: I,
        actor: A,
        actor_type: ActorType,
    ) -> Result<LocalActorRef<A>, ActorRefErr> {
        let _actor_type_name = A::type_name();
        // let span = tracing::trace_span!(
        //     "ActorSystem::new_actor",
        //     actor_type = match actor_type {
        //         ActorType::Anonymous => "Anonymous",
        //         _ => "Tracked",
        //     },
        //     actor_type_name = actor_type_name,
        //     actor_id = id.as_str(),
        // );
        //
        // let _enter = span.enter();

        let id = id.into_actor_id();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let actor_ref = start_actor(
            actor,
            id.clone(),
            actor_type,
            Some(tx),
            Some(self.clone()),
            None,
        );

        if actor_type.is_tracked() {
            let _ = self
                .core
                .scheduler
                .send(RegisterActor {
                    id: id.clone(),
                    actor_ref: actor_ref.clone(),
                })
                .await;
        }

        match rx.await {
            Ok(_) => Ok(actor_ref),
            Err(_e) => {
                error!(
                    "actor not started, actor_id={}, type={}",
                    &id,
                    A::type_name()
                );
                Err(ActorRefErr::ActorStartFailed)
            }
        }
    }

    pub fn is_terminated(&self) -> bool {
        self.core.is_terminated.load(Relaxed)
    }

    pub async fn shutdown(&self) {
        info!("shutting down");

        self.core.is_terminated.store(true, Relaxed);
        let _ = self.core.scheduler.stop().await;

        #[cfg(feature = "remote")]
        if let Some(remote) = &self.core.remote {
            remote.shutdown().await;
        }

        info!("shutdown complete");
    }

    pub async fn get_tracked_actor<A: Actor>(&self, id: ActorId) -> Option<LocalActorRef<A>> {
        let _actor_type_name = A::type_name();
        // let span = tracing::trace_span!(
        //     "ActorSystem::get_tracked_actor",
        //     actor_id = id.as_str(),
        //     actor_type_name
        // );
        // let _enter = span.enter();

        match self.core.scheduler.send(GetActor::new(id)).await {
            Ok(a) => a,
            Err(_) => None,
        }
    }
}

#[cfg(feature = "remote")]
impl ActorSystem {
    pub fn to_remote(&self, remote: RemoteActorSystem) -> Self {
        ActorSystem {
            core: Arc::new(self.core.new_remote(remote)),
        }
    }

    pub fn remote(&self) -> &RemoteActorSystem {
        self.core
            .remote
            .as_ref()
            .expect("this ActorSystem is not setup for remoting")
    }

    pub fn remote_owned(&self) -> RemoteActorSystem {
        self.remote().clone()
    }

    pub fn is_remote(&self) -> bool {
        self.core.remote.is_some()
    }
}

#[cfg(feature = "persistence")]
impl ActorSystem {
    pub fn persistence(&self) -> Option<&Persistence> {
        self.core.persistence.as_ref().map(|p| p.as_ref())
    }

    pub fn to_persistent(&self, persistence: Persistence) -> Self {
        ActorSystem {
            core: Arc::new(self.core.new_persistent(persistence)),
        }
    }
}

impl ActorSystemCore {
    #[cfg(feature = "remote")]
    pub fn new_remote(&self, remote: RemoteActorSystem) -> Self {
        let mut core = self.clone();
        core.remote = Some(remote);

        core
    }

    #[cfg(feature = "persistence")]
    pub fn new_persistent(&self, persistence: Persistence) -> Self {
        let mut core = self.clone();
        core.persistence = Some(Arc::new(persistence));
        core
    }
}
