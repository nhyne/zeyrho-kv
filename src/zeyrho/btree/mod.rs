mod node;
pub mod tree;

const DEGREE: usize = 3;
const SEPARATORS_MAX_SIZE: usize = DEGREE - 1;
const CHILDREN_MAX_SIZE: usize = DEGREE - 1;
const MAX_KVS_IN_LEAF: usize = DEGREE - 1;
