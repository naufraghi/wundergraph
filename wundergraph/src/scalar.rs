use juniper::parser::{ParseError, ScalarToken, Token};
use juniper::{InputValue, ParseScalarResult, ScalarValue, Value};
use serde::de;
use std::fmt;

#[derive(Debug, Clone, PartialEq, ScalarValue)]
pub enum WundergraphScalarValue {
    SmallInt(i16),
    Int(i32),
    BigInt(i64),
    Float(f32),
    Double(f64),
    String(String),
    Boolean(bool),
}

impl ScalarValue for WundergraphScalarValue {
    type Visitor = WundergraphScalarVisitor;

    fn as_int(&self) -> Option<i32> {
        match *self {
            WundergraphScalarValue::SmallInt(ref i) => Some(*i as i32),
            WundergraphScalarValue::Int(ref i) => Some(*i as i32),
            _ => None,
        }
    }

    fn as_string(&self) -> Option<String> {
        match *self {
            WundergraphScalarValue::String(ref s) => Some(s.clone()),
            _ => None,
        }
    }
    fn as_float(&self) -> Option<f64> {
        match *self {
            WundergraphScalarValue::SmallInt(ref i) => Some(*i as f64),
            WundergraphScalarValue::Int(ref i) => Some(*i as f64),
            WundergraphScalarValue::BigInt(ref i) => Some(*i as f64),
            WundergraphScalarValue::Float(ref f) => Some(*f as f64),
            WundergraphScalarValue::Double(ref f) => Some(*f as f64),
            _ => None,
        }
    }
    fn as_boolean(&self) -> Option<bool> {
        match *self {
            WundergraphScalarValue::Boolean(ref b) => Some(*b),
            _ => None,
        }
    }
}

impl<'a> From<&'a str> for WundergraphScalarValue {
    fn from(s: &'a str) -> Self {
        WundergraphScalarValue::String(s.into())
    }
}

#[cfg(feature = "chrono")]
impl From<chrono::NaiveDateTime> for WundergraphScalarValue {
    fn from(n: chrono::NaiveDateTime) -> Self {
        WundergraphScalarValue::Double(n.timestamp() as _)
    }
}


#[doc(hidden)]
#[derive(Default, Debug, Clone, Copy)]
pub struct WundergraphScalarVisitor;

impl<'de> de::Visitor<'de> for WundergraphScalarVisitor {
    type Value = WundergraphScalarValue;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a valid input value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(WundergraphScalarValue::Boolean(value))
    }

    fn visit_i16<E>(self, value: i16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(WundergraphScalarValue::SmallInt(value))
    }

    fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value <= i16::max_value() as i32 {
            self.visit_i16(value as i16)
        } else {
            Ok(WundergraphScalarValue::Int(value))
        }
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value <= i32::max_value() as i64 {
            self.visit_i32(value as i32)
        } else {
            Ok(WundergraphScalarValue::BigInt(value))
        }
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value <= i32::max_value() as u32 {
            self.visit_i32(value as i32)
        } else {
            self.visit_u64(u64::from(value))
        }
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value <= i64::max_value() as u64 {
            self.visit_i64(value as i64)
        } else {
            // Browser's JSON.stringify serialize all numbers having no
            // fractional part as integers (no decimal point), so we
            // must parse large integers as floating point otherwise
            // we would error on transferring large floating point
            // numbers.
            Ok(WundergraphScalarValue::Double(value as f64))
        }
    }

    fn visit_f32<E>(self, value: f32) -> Result<Self::Value, E> {
        Ok(WundergraphScalarValue::Float(value))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
        Ok(WundergraphScalarValue::Double(value))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_string(value.into())
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(WundergraphScalarValue::String(value))
    }
}

graphql_scalar!(i64 as "BigInt" where Scalar = WundergraphScalarValue {
    resolve(&self) -> Value {
        Value::scalar(*self)
    }

    from_input_value(v: &InputValue) -> Option<i64> {
        match *v {
            InputValue::Scalar(WundergraphScalarValue::SmallInt(i)) => Some(i64::from(i)),
            InputValue::Scalar(WundergraphScalarValue::Int(i)) => Some(i64::from(i)),
            InputValue::Scalar(WundergraphScalarValue::BigInt(i)) => Some(i),
            _ => None,
        }
    }

    from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, WundergraphScalarValue> {
        if let ScalarToken::Int(v) = value {
                v.parse()
                    .map_err(|_| ParseError::UnexpectedToken(Token::Scalar(value)))
                    .map(|s: i64| s.into())
        } else {
                Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
});

graphql_scalar!(i16 as "SmallInt" where Scalar = WundergraphScalarValue {
    resolve(&self) -> Value {
        Value::scalar(*self)
    }

    from_input_value(v: &InputValue) -> Option<i16> {
        match *v {
            InputValue::Scalar(WundergraphScalarValue::SmallInt(i)) => Some(i),
            _ => None,
        }
    }

    from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, WundergraphScalarValue> {
        if let ScalarToken::Int(v) = value {
                v.parse()
                    .map_err(|_| ParseError::UnexpectedToken(Token::Scalar(value)))
                    .map(|s: i16| s.into())
        } else {
                Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
    });

graphql_scalar!(f32 as "SmallFloat" where Scalar = WundergraphScalarValue {
    resolve(&self) -> Value {
        Value::scalar(*self)
    }

    from_input_value(v: &InputValue) -> Option<f32> {
        match *v {
            InputValue::Scalar(WundergraphScalarValue::SmallInt(i)) => Some(f32::from(i)),
            InputValue::Scalar(WundergraphScalarValue::Float(i)) => Some(i),
            _ => None,
        }
    }

    from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, WundergraphScalarValue> {
        if let ScalarToken::Int(v) = value {
                v.parse()
                    .map_err(|_| ParseError::UnexpectedToken(Token::Scalar(value)))
                    .map(|s: f32| s.into())
        } else {
                Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
    });
