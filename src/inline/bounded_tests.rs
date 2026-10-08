use super::{hashmap::InlineHashMap, vec::InlineVec};

#[test]
fn vector_clone_and_remove_preserve_prefix() {
    let mut values = InlineVec::<_, 4>::new();
    values.push(10).unwrap();
    values.push(20).unwrap();
    values.push(30).unwrap();
    let cloned = values.clone();
    assert_eq!(values.remove(1), 20);
    assert_eq!(values.as_slice(), &[10, 30]);
    assert_eq!(cloned.as_slice(), &[10, 20, 30]);
}

#[test]
fn map_replacement_clone_and_remove() {
    let mut values = InlineHashMap::<_, _, 2>::new();
    values.insert(1, 10).unwrap();
    values.insert(1, 20).unwrap();
    assert_eq!(values.len(), 1);
    let cloned = values.clone();
    assert_eq!(values.remove(&1), Some(20));
    assert_eq!(cloned.get(&1), Some(&20));
}

#[cfg(not(feature = "std"))]
#[test]
fn capacity_errors_leave_existing_values_intact() {
    let mut vector = InlineVec::<_, 1>::new();
    vector.push(10).unwrap();
    assert_eq!(
        vector.push(20),
        Err(crate::ParseError::ChildCapacityExceeded)
    );
    assert_eq!(vector.as_slice(), &[10]);
    let mut map = InlineHashMap::<_, _, 1>::new();
    map.insert(1, 10).unwrap();
    assert_eq!(
        map.insert(2, 20),
        Err(crate::ParseError::AttributeCapacityExceeded)
    );
    map.insert(1, 30).unwrap();
    assert_eq!(map.get(&1), Some(&30));
    assert_eq!(map.len(), 1);
}
