use coerce_rt::actor::context::{ActorContext, ActorHandlerContext};
use coerce_rt::actor::message::{Handler, Message};
use coerce_rt::actor::Actor;
use coerce_rt::worker::{Worker, WorkerRefExt};

#[macro_use]
extern crate async_trait;

#[derive(Clone)]
pub struct MyWorker {}

pub struct HeavyTask;

impl Actor for MyWorker {}

impl Message for HeavyTask {
    type Result = &'static str;
}

#[async_trait]
impl Handler<HeavyTask> for MyWorker {
    async fn handle(&mut self, message: HeavyTask, ctx: &mut ActorHandlerContext) -> &'static str {
        // do some IO with a connection pool attached to `MyWorker`?

        "my_result"
    }
}

#[tokio::test]
pub async fn test_workers() {
    let mut context = ActorContext::new();

    let state = MyWorker {};
    let mut worker = Worker::new(state, 4, &mut context).await.unwrap();

    assert_eq!(worker.dispatch(HeavyTask).await, Ok("my_result"));
}