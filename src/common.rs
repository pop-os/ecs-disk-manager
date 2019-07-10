pub fn remove_item<T: PartialEq>(vector: &mut Vec<T>, item: &T) -> Option<T> {
    if let Some(id) = vector.iter().position(|v| v == item) {
        return Some(vector.remove(id));
    }

    None
}
