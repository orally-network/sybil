use core::hash::Hash;
use std::hash::Hasher;

use serde_json::Value;

// trait Hashable {
//     fn hash(&self) -> u64;
// };


// impl Hashable for Value {
//     fn hash(&self) -> u64 {
//         match self {
//             Value::Null => 0,
//             Value::Bool(b) => {
//                 if *b {
//                     11
//                 } else {
//                     12
//                 }
//             }
//             Value::Number(n) => {
//                 if n.is_i64() {
//                     n.as_i64().unwrap() as u64
//                 } else {
//                     n.as_f64().unwrap() as u64
//                 }
//             }
//             Value::String(s) => {
//                 let mut hasher = std::collections::hash_map::DefaultHasher::new();
//                 s.hash(&mut hasher);
//                 hasher.finish()
//             }
//             Value::Array(_) => {
//                 let mut hasher = std::collections::hash_map::DefaultHasher::new();
//                 self.to_string().hash(&mut hasher);
//                 hasher.finish()
//             }
//             Value::Object(_) => {
//                 let mut hasher = std::collections::hash_map::DefaultHasher::new();
//                 self.to_string().hash(&mut hasher);
//                 hasher.finish()
//             }
//         }
//     }
// }




pub fn find_most_frequent_value<T: PartialEq + Clone + Eq + Hash>(arr: &[T]) -> Option<&T> {
    let map = arr.iter().fold(std::collections::HashMap::new(), |mut acc, x| {
        *acc.entry(x).or_insert(0) += 1;
        acc
    });

    let max_value = map.iter().max_by(|(_, v1), (_, v2)| v1.cmp(v2)).map(|(k, _)| k).cloned();

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
