use secstr::SecStr;

#[derive(Debug, Clone, Shrinkwrap)]
pub struct LuksPassphrase(SecStr);

impl From<SecStr> for LuksPassphrase {
    fn from(string: SecStr) -> LuksPassphrase { LuksPassphrase(string) }
}
