mod node;
pub mod tree;

const DEGREE: usize = 3;
const SEPARATORS_MAX_SIZE: usize = DEGREE - 1;
const CHILDREN_MAX_SIZE: usize = DEGREE;
const MAX_KVS_IN_LEAF: usize = DEGREE - 1;
const MIN_ELEMENTS_IN_LEAF: usize = DEGREE / 2;
const MIN_ELEMENTS_IN_LEAF_MINUS_ONE: i32 = (DEGREE / 2 - 1) as i32;
