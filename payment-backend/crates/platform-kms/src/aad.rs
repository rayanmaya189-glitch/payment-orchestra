/// Additional Authenticated Data binding helpers.
pub fn build_aad(link_id: &uuid::Uuid) -> Vec<u8> {
    link_id.as_bytes().to_vec()
}
