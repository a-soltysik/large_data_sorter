pub fn merge_sort<T>(slice: &[T]) -> Vec<T>
    where
        T: PartialOrd + Clone
{
    match slice.len() {
        0..=1 => slice.to_vec(),
        2 => {
            if slice[0] > slice[1] {
                return vec![slice[1].clone(), slice[0].clone()]
            }
            slice.to_vec()
        }
        _ => {
            let middle = slice.len() / 2;
            let left = merge_sort(&slice[0..middle]);
            let right = merge_sort(&slice[middle..]);
            merge(&left, &right)
        }
    }
}

fn merge<T>(left: &[T], right: &[T]) -> Vec<T>
    where
        T: PartialOrd + Clone
{
    let mut left_pos = 0;
    let mut right_pos = 0;
    let mut merged: Vec<T> = Vec::new();
    merged.reserve(left.len() + right.len());

    while left_pos != left.len() && right_pos < right.len() {
        if left[left_pos] < right[right_pos] {
            merged.push(left[left_pos].clone());
            left_pos += 1;
        } else {
            merged.push(right[right_pos].clone());
            right_pos += 1
        }
    }

    if left_pos < left.len() {
        merged.extend_from_slice(&left[left_pos..]);
    }

    if right_pos < right.len() {
        merged.extend_from_slice(&right[right_pos..]);
    }

    return merged;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge() {
        let merged = merge([1, 4, 8, 10].as_slice(), [2, 3, 8, 9].as_slice());
        assert_eq!(merged, vec![1, 2, 3, 4, 8, 8, 9, 10]);
    }

    #[test]
    fn test_merge_sort() {
        let unsorted = vec![5, 1, 9, 10, 3, 45, 2, 4, 4, 12];
        let mut sorted = unsorted.clone();
        sorted.sort();
        assert_eq!(merge_sort(&unsorted), sorted)
    }
}