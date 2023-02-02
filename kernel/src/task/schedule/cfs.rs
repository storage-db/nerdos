use super::rbt::*;

#[cfg(test)]
fn init() {
    let mut vec = create_vec(4, 4, 5);

    let mut tree = RBTree::<i32, u32, 4, 4>::init_slice(vec.as_mut_slice()).unwrap();
    assert!(tree.is_empty());

    assert_eq!(tree.insert(12, 32), Ok(None));
    assert_eq!(tree.get(&12), Some(32));
    assert_eq!(tree.len(), 1);

    assert_eq!(tree.insert(32, 44), Ok(None));
    assert_eq!(tree.get(&32), Some(44));
    assert_eq!(tree.len(), 2);

    assert_eq!(tree.insert(123, 321), Ok(None));
    assert_eq!(tree.get(&123), Some(321));
    assert_eq!(tree.len(), 3);

    assert_eq!(tree.insert(123, 322), Ok(Some(321)));
    assert_eq!(tree.get(&123), Some(322));
    assert_eq!(tree.len(), 3);

    assert_eq!(tree.insert(14, 32), Ok(None));
    assert_eq!(tree.get(&14), Some(32));
    assert_eq!(tree.len(), 4);

    assert_eq!(tree.insert(1, 2), Ok(None));
    assert_eq!(tree.insert(1, 4), Ok(Some(2)));
    assert_eq!(tree.insert(3, 4), Err(Error::NoNodesLeft));

    assert_eq!(tree.get(&15), None);

    assert_eq!(tree.len(), 5);
}


