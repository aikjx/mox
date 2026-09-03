fpath = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\cloud\svc\mox-cloud-filer-svc\src\filer_server.rs'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

# Fix put: clone logical for async move
old = '''    fn put(&self, bucket: &str, key: &str, data: &[u8]) -> FilerResult<()> {
        let logical = Self::logical_key(bucket, key);
        let client = self.client.clone();
        let data_vec = data.to_vec();
        self.rt
            .block_on(async move {
                client
                    .put_object(&logical, "application/octet-stream", &data_vec)
                    .await
            })
            .map_err(|e| FilerError::Other(format!("S3 PUT {logical} 失败: {e}")))
    }'''
new = '''    fn put(&self, bucket: &str, key: &str, data: &[u8]) -> FilerResult<()> {
        let logical = Self::logical_key(bucket, key);
        let logical_mv = logical.clone();
        let client = self.client.clone();
        let data_vec = data.to_vec();
        self.rt
            .block_on(async move {
                client
                    .put_object(&logical_mv, "application/octet-stream", &data_vec)
                    .await
            })
            .map_err(|e| FilerError::Other(format!("S3 PUT {logical} 失败: {e}")))
    }'''
if old in content:
    content = content.replace(old, new, 1)
    print('Fixed put')

# Fix get
old = '''    fn get(&self, bucket: &str, key: &str) -> FilerResult<Vec<u8>> {
        let logical = Self::logical_key(bucket, key);
        let client = self.client.clone();
        self.rt
            .block_on(async move { client.get_object(&logical).await })
            .map_err(|e| match e {
                mox_base_store_core::StoreError::NotFound { .. } => FilerError::NotFound,
                other => FilerError::Other(format!("S3 GET {logical} 失败: {other}")),
            })
    }'''
new = '''    fn get(&self, bucket: &str, key: &str) -> FilerResult<Vec<u8>> {
        let logical = Self::logical_key(bucket, key);
        let logical_mv = logical.clone();
        let client = self.client.clone();
        self.rt
            .block_on(async move { client.get_object(&logical_mv).await })
            .map_err(|e| match e {
                mox_base_store_core::StoreError::NotFound { .. } => FilerError::NotFound,
                other => FilerError::Other(format!("S3 GET {logical} 失败: {other}")),
            })
    }'''
if old in content:
    content = content.replace(old, new, 1)
    print('Fixed get')

# Fix list
old = '''    fn list(&self, bucket: &str) -> FilerResult<Vec<String>> {
        let prefix = format!("{}/", bucket.trim_matches('/'));
        let client = self.client.clone();
        let keys = self
            .rt
            .block_on(async move { client.list_objects(&prefix).await })
            .map_err(|e| FilerError::Other(format!("S3 LIST {prefix} 失败: {e}")))?;
        let out: Vec<String> = keys
            .into_iter()
            .filter_map(|k| k.strip_prefix(&prefix).map(|s| s.to_string()))
            .collect();
        Ok(out)
    }'''
new = '''    fn list(&self, bucket: &str) -> FilerResult<Vec<String>> {
        let prefix = format!("{}/", bucket.trim_matches('/'));
        let prefix_mv = prefix.clone();
        let client = self.client.clone();
        let keys = self
            .rt
            .block_on(async move { client.list_objects(&prefix_mv).await })
            .map_err(|e| FilerError::Other(format!("S3 LIST {prefix} 失败: {e}")))?;
        let out: Vec<String> = keys
            .into_iter()
            .filter_map(|k| k.strip_prefix(&prefix).map(|s| s.to_string()))
            .collect();
        Ok(out)
    }'''
if old in content:
    content = content.replace(old, new, 1)
    print('Fixed list')

# Fix delete
old = '''    fn delete(&self, bucket: &str, key: &str) -> FilerResult<()> {
        let logical = Self::logical_key(bucket, key);
        let client = self.client.clone();
        self.rt
            .block_on(async move { client.delete_object(&logical).await })
            .map_err(|e| FilerError::Other(format!("S3 DELETE {logical} 失败: {e}")))
    }'''
new = '''    fn delete(&self, bucket: &str, key: &str) -> FilerResult<()> {
        let logical = Self::logical_key(bucket, key);
        let logical_mv = logical.clone();
        let client = self.client.clone();
        self.rt
            .block_on(async move { client.delete_object(&logical_mv).await })
            .map_err(|e| FilerError::Other(format!("S3 DELETE {logical} 失败: {e}")))
    }'''
if old in content:
    content = content.replace(old, new, 1)
    print('Fixed delete')

# Fix head
old = '''    fn head(&self, bucket: &str, key: &str) -> FilerResult<u64> {
        let logical = Self::logical_key(bucket, key);
        let client = self.client.clone();
        let info = self
            .rt
            .block_on(async move { client.head_object(&logical).await })
            .map_err(|e| FilerError::Other(format!("S3 HEAD {logical} 失败: {e}")))?;'''
new = '''    fn head(&self, bucket: &str, key: &str) -> FilerResult<u64> {
        let logical = Self::logical_key(bucket, key);
        let logical_mv = logical.clone();
        let client = self.client.clone();
        let info = self
            .rt
            .block_on(async move { client.head_object(&logical_mv).await })
            .map_err(|e| FilerError::Other(format!("S3 HEAD {logical} 失败: {e}")))?;'''
if old in content:
    content = content.replace(old, new, 1)
    print('Fixed head')

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)

print('Done')
