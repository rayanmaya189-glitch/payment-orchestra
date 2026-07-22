pub struct VaultClient {
    pub addr: String,
    pub token: String,
    client: reqwest::Client,
}

impl VaultClient {
    pub fn new(addr: &str, token: &str) -> Self {
        Self {
            addr: addr.to_string(),
            token: token.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn read_secret(&self, path: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}/v1/{}", self.addr, path);
        let resp = self.client.get(&url)
            .header("X-Vault-Token", &self.token)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        Ok(body)
    }
}
