pub async fn read_asset_bytes<'a, 'b>(&'a self, path: PathBuf) -> Result<Vec<u8>, LoadError> {
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .open(path)?;

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    Ok(bytes)
}