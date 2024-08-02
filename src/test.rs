use std::hash::Hasher as _;

use crate::{Revisable, RevisionHash, RevisionHasher};

struct TestInt(i32);

impl Revisable for TestInt {
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = RevisionHasher::new();
        hasher.write_i32(self.0);
        hasher.into_revision()
    }
}

#[test]
fn basic_test() {
    let mut x = TestInt(1);

    let r0 = x.get_revision();

    x.0 = 2;

    let r1 = x.get_revision();

    assert_ne!(r0, r1);

    let y = TestInt(2);

    let r2 = y.get_revision();

    assert_eq!(r1, r2);
}

// TODO: more tests
