use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    custom_keyword,
    parse::{Parse, ParseStream},
    Ident, LitBool, Token, Type,
};

macro_rules! attribute_parser {
    (@field_parser $input_in_parentheses:ident $field_name:ident Option<$nested_field_type:ty>) => {
        custom_keyword!($field_name);
        let $field_name: Option<$nested_field_type> = if input_in_parentheses.peek($field_name) {
            $input_in_parentheses.parse::<$field_name>()?;
            $input_in_parentheses.parse::<syn::Token![=]>()?;
            let $field_name = $input_in_parentheses.parse()?;
            if !$input_in_parentheses.is_empty() {
                $input_in_parentheses.parse::<syn::Token![,]>()?;
            }
            Some($field_name)
        } else {
            None
        };
    };

    (@field_parser $input_in_parentheses:ident $field_name:ident $field_type:ty) => {
        custom_keyword!($field_name);
        $input_in_parentheses.parse::<$field_name>()?;
        $input_in_parentheses.parse::<syn::Token![=]>()?;
        let $field_name: $field_type = $input_in_parentheses.parse()?;
        if !$input_in_parentheses.is_empty() {
            $input_in_parentheses.parse::<syn::Token![,]>()?;
        }
    };

    (@field_parsers $input_in_parentheses:ident $($field_name:ident: $field_type:ty),+) => {
        $(
            attribute_parser!(@field_parser $input_in_parentheses $field_name $field_type);
        )+
    };

    (@initializer $variant:ident $($field_name:ident: $field_type:ty),+) => {
        Attribute::$variant {
            $($field_name),+
        }
    };

    (@display_fields $variant:ident $($field_name:ident: $field_type:ty),+) => {
        [
            $(format!(concat!(stringify!($field_name), " = {}"), $field_name.to_token_stream()),)+
        ]
    };

    (pub enum Attribute { $($variant:ident { $($field:tt)+ },)+ }) => {
        #[derive(Clone, Debug)]
        pub enum Attribute {
            $($variant { $($field)+ },)+
        }

        impl std::fmt::Display for Attribute {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                use convert_case::Casing;
                use quote::ToTokens;

                match self {
                    $(attribute_parser!(@initializer $variant $($field)+) => {
                        let variant = stringify!($variant).to_string().to_case(convert_case::Case::Snake);
                        write!(formatter, "#[{}(", variant)?;
                        formatter.write_str(&attribute_parser!(@display_fields variant $($field)+).join(", "))?;
                        formatter.write_str(")]")?;
                    },)+
                }

                Ok(())
            }
        }

        impl syn::parse::Parse for Attribute {
            fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
                use convert_case::Casing;

                input.parse::<syn::Token![#]>()?;
                let input_in_brackets;
                syn::bracketed!(input_in_brackets in input);
                let name: syn::Ident = input_in_brackets.parse()?;
                let name_camel_case = name.to_string().to_case(convert_case::Case::Pascal);
                let input_in_parentheses;
                syn::parenthesized!(input_in_parentheses in input_in_brackets);
                match name_camel_case.as_str() {
                    $(stringify!($variant) => {
                        attribute_parser!(@field_parsers input_in_parentheses $($field)+);
                        Ok(attribute_parser!(@initializer $variant $($field)+))
                    },)+
                    _ => Err(syn::Error::new(
                        name.span(),
                        format!("Unexpected attribute {name}"),
                    ))
                }
            }
        }
    };
}

attribute_parser! {
    pub enum Attribute {
        AdditionalOutput { data_type: Type, name: Ident, path: Path },
        HardwareInterface { name: Ident },
        HistoricInput { data_type: Type, name: Ident, path: Path },
        Input { cycler_instance: Option<Ident>, data_type: Type, is_required: LitBool, name: Ident, path: Path },
        MainOutput { data_type: Type, name: Ident },
        Parameter { data_type: Type, name: Ident, path: Path },
        PerceptionInput { cycler_instance: Ident, data_type: Type, name: Ident, path: Path },
        PerceptionModule { cycler_module: Ident },
        PersistentState { data_type: Type, name: Ident, path: Path },
        RealtimeModule { cycler_module: Ident },
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Path {
    pub segments: Vec<Ident>,
}

impl Parse for Path {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut segments = vec![];
        loop {
            segments.push(input.parse()?);
            if !input.peek(Token![.]) {
                break;
            }
            input.parse::<Token![.]>()?;
        }
        Ok(Self { segments })
    }
}

impl ToTokens for Path {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for (index, segment) in self.segments.iter().enumerate() {
            segment.to_tokens(tokens);
            if index + 1 < self.segments.len() {
                <Token![.]>::default().to_tokens(tokens);
            }
        }
    }
}