#![allow(dead_code)]
use curl::easy::Easy;

pub fn http_get(url: &str) -> Option<String> {
    let mut vec = Vec::new();
    let mut easy = Easy::new();
    easy.url(url).ok()?;
    {
        let mut transfer = easy.transfer();
        let _ = transfer.write_function(|data| {
            vec.extend_from_slice(data);
            Ok(data.len())
        });
        transfer.perform().ok()?;
    }
    String::from_utf8(vec).ok()
}

pub fn join_with<S, I, T>(mut iter: I, sep: S) -> String
where
    S: AsRef<str>,
    T: AsRef<str>,
    I: Iterator<Item = T>,
{
    let mut buf = String::new();
    if let Some(s) = iter.next() {
        buf.push_str(s.as_ref());
    }
    for i in iter {
        buf.push_str(sep.as_ref());
        buf.push_str(i.as_ref());
    }
    buf
}

pub(crate) mod testing {
    pub fn init_logger() {
        let _ = env_logger::Builder::from_default_env()
            .default_format_timestamp(false)
            .try_init();
    }
}
