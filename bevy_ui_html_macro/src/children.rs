use proc_macro2::{Punct, Spacing, TokenStream};
use quote::{ToTokens, quote};

pub fn punctuate_bsn_children(children: &Vec<TokenStream>) -> TokenStream {
    let mut children_stream = TokenStream::new();
    let mut iter = children.into_iter().peekable();
    let first = Punct::new('-', Spacing::Joint);
    let second = Punct::new('-', Spacing::Alone);
    while let Some(child) = iter.next() {
        child.to_tokens(&mut children_stream);
        if iter.peek().is_some() {
            children_stream.extend(quote! {
                #first #second
            });
        }
    }
    children_stream
}
