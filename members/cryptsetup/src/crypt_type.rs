pub struct CryptTypeStr<'a>(pub(crate) &'a str);

impl<'a> CryptTypeStr<'a> {
    pub fn as_str(&self) -> &str { self.0 }
}

#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
pub enum CryptType {
    Luks1,
    Luks2,
}

impl<'a> From<CryptTypeStr<'a>> for CryptType {
    fn from(string: CryptTypeStr) -> Self {
        match string.as_str() {
            "LUKS1" => CryptType::Luks1,
            "LUKS2" => CryptType::Luks2,
            string => panic!("unknown type string: {}", string),
        }
    }
}

impl From<CryptType> for CryptTypeStr<'static> {
    fn from(t: CryptType) -> Self {
        let string = match t {
            CryptType::Luks1 => "LUKS1",
            CryptType::Luks2 => "LUKS2",
        };

        CryptTypeStr(string)
    }
}
