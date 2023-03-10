pub fn hostname() -> String {
    // Linux
    let data = std::fs::read_to_string("/etc/hostname").unwrap();
    let hostname = data.trim_end().to_string();
    if let Ok(user) = std::env::var("USER") {
        return format!("{}-{}",user.to_ascii_uppercase(),hostname.to_ascii_uppercase());
    }
    hostname
}