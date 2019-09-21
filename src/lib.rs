pub mod mdl;
pub mod md2;

#[allow(non_camel_case_types)]
type vec3_t = [f32; 3];

#[derive(Debug)]
pub struct Error {
    desc: String,
    source: Option<std::io::Error>,
}

type Result<T> = std::result::Result<T, Error>;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.desc)
    }
}

impl std::error::Error for Error {}

impl Error {
    fn ident(actual: i32, expected: i32) -> Self {
        Error {
            desc: format!(
                "unexpectd ident value. expected: {}. actual: {}",
                expected,
                actual
            ),
            source: None,
        }
    }

    fn version(actual: i32, expected: i32) -> Self {
        Error {
            desc: format!("unexpected version value. expected: {}. actual: {}", expected, actual),
            source: None,
        }
    }

    fn io(src: std::io::Error, msg: &str) -> Self {
        Error {
            desc: format!("io error: {}. message: {}", &src, msg),
            source: Some(src),
        }
    }

    fn utf8(src: std::str::Utf8Error, msg: &str) -> Self {
        Error {
            desc: format!("utf8 error: {}. message: {}", src, msg),
            source: None,
        }
    }

    fn unsupported(msg: &str) -> Self {
        Error {
            desc: format!("{}", msg),
            source: None,
        }
    }
}

fn to_utf8(bytes: &[u8]) -> std::result::Result<String, std::str::Utf8Error> {
    let utf_str = if let Some(idx) = bytes.iter().enumerate().find(|(_, v)| **v == 0) {
        std::str::from_utf8(&bytes[0..idx.0])?
    } else {
        std::str::from_utf8(&bytes)?
    };

    Ok(utf_str.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
