use super::rbt::*;

#[allow(dead_code)]
#[derive(Debug)]
pub struct CFSchedulerState{
    vtime : usize,
}

pub struct CFScheduler{
    rbt : RBTree,
}


impl CFSchedulerTrait for CFScheduler{
    fn new() -> Self{
        // let mut  vec  = crate_vec()
    }
}