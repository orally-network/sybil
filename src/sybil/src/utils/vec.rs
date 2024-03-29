use core::hash::Hash;
use std::{iter::Sum, ops::Div};

pub fn find_average<'a, T: Sum<&'a T> + Div<Output = T> + From<u32>>(arr: &'a [T]) -> T {
    let sum: T = arr.into_iter().sum();
    let count = arr.len() as u32;

    return sum / count.into();
}

pub fn find_most_frequent_value<T: PartialEq + Clone + Eq + Hash>(arr: &[T]) -> Option<&T> {
    let map = arr
        .iter()
        .fold(std::collections::HashMap::new(), |mut acc, x| {
            *acc.entry(x).or_insert(0) += 1;
            acc
        });

    let max_value = map
        .iter()
        .max_by(|(_, v1), (_, v2)| v1.cmp(v2))
        .map(|(k, _)| k)
        .cloned();

    max_value
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_find_mind_frequent_value() {
        let mut arr1 = vec![2, 5, 7, 4, 4, 9, 5, 3, 4];

        arr1.sort();

        let value = super::find_most_frequent_value(&arr1);
        println!("{:?}", value);
        assert_eq!(value, Some(&4));
    }
}
