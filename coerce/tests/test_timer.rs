use coerce::actor::context::{ActorContext, ActorSystem};
use coerce::actor::scheduler::timer::{Timer, TimerTick};

use crate::util::*;
use coerce::actor::message::{Handler, Message};
use std::time::Duration;

pub mod util;

#[macro_use]
extern crate serde;
extern crate serde_json;

extern crate chrono;

#[macro_use]
extern crate async_trait;

#[derive(Clone)]
pub struct TestTimer {}

impl Message for TestTimer {
    type Result = ();
}

impl TimerTick for TestTimer {}

#[async_trait]
impl Handler<TestTimer> for TestActor {
    async fn handle(&mut self, _message: TestTimer, _ctx: &mut ActorContext) {
        self.counter += 1;
    }
}

#[tokio::test]
pub async fn test_timer() {
    let mut actor_ref = ActorSystem::new()
        .new_tracked_actor(TestActor::new())
        .await
        .unwrap();

    let timer = Timer::start(actor_ref.clone(), Duration::from_millis(11), TestTimer {});

    tokio::time::sleep(Duration::from_millis(40)).await;
    let counter_40ms_later = actor_ref.exec(|a| a.counter).await;

    tokio::time::sleep(Duration::from_millis(40)).await;
    let counter_80ms_later = actor_ref.exec(|a| a.counter).await;

    let stop = timer.stop();

    let counter_now = actor_ref.exec(|a| a.counter).await.unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;

    let counter_changed_after_stop = counter_now != actor_ref.exec(|a| a.counter).await.unwrap();

    assert_eq!(counter_40ms_later, Ok(4));
    assert_eq!(counter_80ms_later, Ok(8));
    assert_eq!(stop, true);
    assert_eq!(counter_changed_after_stop, false);
}