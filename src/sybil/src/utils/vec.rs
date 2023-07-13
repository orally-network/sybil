pub fn find_most_frequent_value<T: PartialEq + Clone>(arr: &[T]) -> Option<T> {
    if arr.is_empty() {
        return None;
    }

    if arr.len() == 1 {
        return Some(arr[0].clone());
    }

    let mut current_value = arr[0].clone();
    let mut current_count = 1;
    let mut max_value = arr[0].clone();
    let mut max_count = 1;

    for v in arr.iter().skip(1) {
        if *v == current_value {
            current_count += 1;
        } else {
            if current_count > max_count {
                max_value = current_value;
                max_count = current_count;
            }
            current_value = v.clone();
            current_count = 1;
        }
    }

    if current_count > max_count {
        max_value = current_value;
    }

    Some(max_value)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_find_mind_frequent_value() {
        let mut arr1 = vec![2, 5, 7, 4, 4, 9, 5, 3, 4];

        arr1.sort();

        let value = super::find_most_frequent_value(&arr1);
        println!("{:?}", value);
        assert_eq!(value, Some(4));
    }
}
