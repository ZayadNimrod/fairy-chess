

impl From<MoveCompact> for String {
    fn from(m: MoveCompact) -> Self {
        m.notation()
    }
}

impl TryFrom<String> for MoveCompact {
    type Error = parser::ParsingError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        parser::parse_string(&value)
    }
}