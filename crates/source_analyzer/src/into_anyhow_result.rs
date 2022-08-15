use std::{fmt::Display, path::Path};

use anyhow::anyhow;
use proc_macro2::Span;
use syn::Error;

pub fn into_anyhow_result<T, P>(file_path: P, result: syn::Result<T>) -> anyhow::Result<T>
where
    P: AsRef<Path>,
{
    result.map_err(|error| {
        let start = error.span().start();
        anyhow!(
            "{error} at {}:{}:{}",
            file_path.as_ref().display(),
            start.line,
            start.column
        )
    })
}

pub trait SynContext<T, E> {
    fn syn_context<P>(self, file_path: P) -> anyhow::Result<T>
    where
        P: AsRef<Path>;
}

impl<T> SynContext<T, Error> for syn::Result<T> {
    fn syn_context<P>(self, file_path: P) -> anyhow::Result<T>
    where
        P: AsRef<Path>,
    {
        self.map_err(|error| {
            let start = error.span().start();
            anyhow!(
                "{error} at {}:{}:{}",
                file_path.as_ref().display(),
                start.line,
                start.column
            )
        })
    }
}

pub fn new_syn_error_as_anyhow_result<T, M, P>(
    span: Span,
    message: M,
    file_path: P,
) -> anyhow::Result<T>
where
    M: Display,
    P: AsRef<Path>,
{
    Err(Error::new(span, message)).syn_context(file_path)
}