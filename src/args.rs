use std::convert::Infallible;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
	#[error("failed to parse integer: {0}")]
	ParseIntError(#[from] std::num::ParseIntError),
	#[error("failed to parse float: {0}")]
	ParseFloatError(#[from] std::num::ParseFloatError),
	#[error("expected 'true' or 'false', got '{0}'")]
	InvalidBool(String),
	#[error("invalid value: {0}")]
	InvalidValue(String),
}

pub trait ArgParser {
	type Output;
	type Error;

	fn parse(&self, input: &str) -> Result<Self::Output, Self::Error>;
}

#[derive(Default)]
pub struct StringParser;

impl ArgParser for StringParser {
	type Output = String;
	type Error = Infallible;

	fn parse(&self, input: &str) -> Result<Self::Output, Self::Error> {
		Ok(input.to_owned())
	}
}

macro_rules! impl_int_parser {
    ($($ty:ty => $name:ident),* $(,)?) => {
        $(
            #[derive(Default)]
            pub struct $name;

            impl ArgParser for $name {
                type Output = $ty;
                type Error = ParseError;

                fn parse(&self, input: &str) -> Result<Self::Output, Self::Error> {
                    input.parse().map_err(ParseError::ParseIntError)
                }
            }
        )*
    };
}

impl_int_parser!(
	i8 => I8Parser,
	i16 => I16Parser,
	i32 => I32Parser,
	i64 => I64Parser,
	u8 => U8Parser,
	u16 => U16Parser,
	u32 => U32Parser,
	u64 => U64Parser,
);

macro_rules! impl_float_parser {
    ($($ty:ty => $name:ident),* $(,)?) => {
        $(
            #[derive(Default)]
            pub struct $name;

            impl ArgParser for $name {
                type Output = $ty;
                type Error = ParseError;

                fn parse(&self, input: &str) -> Result<Self::Output, Self::Error> {
                    input.parse().map_err(ParseError::ParseFloatError)
                }
            }
        )*
    };
}

impl_float_parser!(
	f32 => F32Parser,
	f64 => F64Parser,
);

#[derive(Default)]
pub struct BoolParser;

impl ArgParser for BoolParser {
	type Output = bool;
	type Error = ParseError;

	fn parse(&self, input: &str) -> Result<Self::Output, Self::Error> {
		match input {
			"true" => Ok(true),
			"false" => Ok(false),
			value => Err(ParseError::InvalidBool(value.to_owned())),
		}
	}
}
