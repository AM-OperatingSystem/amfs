#![allow(unknown_lints)]
#![allow(require_stability_comment)]

use proc_macro::TokenStream;

use proc_macro2::{Ident, Span};

#[macro_use]
extern crate quote;

#[cfg(not(tarpaulin_include))]
#[proc_macro]
pub fn generate_image(input: TokenStream) -> TokenStream {
    let item: syn::LitInt = syn::parse(input).expect("failed to parse input");
    let num_to_gen = item.base10_parse::<usize>().unwrap();
    let filename = format!("test_{:04}.img", num_to_gen);
    let generate_fn = Ident::new(
        format!("generate_{:04}", num_to_gen).as_str(),
        Span::call_site(),
    );
    let output = quote! {
        use std::fs::OpenOptions;
        let file = OpenOptions::new().read(true).write(true).create(true).open(#filename).unwrap();

        #generate_fn(&file);

        let mut file = OpenOptions::new().read(true).open(#filename).unwrap();

        let expected = amfs_tests::imagegen::get_checksums();

        use data_encoding::HEXUPPER;
        use sha2::{Sha256, Digest};
        let mut sha256 = Sha256::new();
        std::io::copy(&mut file, &mut sha256).unwrap();
        let digest = sha256.finalize();
        assert_eq!(HEXUPPER.encode(digest.as_ref()),amfs_tests::imagegen::get_checksums()[#num_to_gen]);
    };
    output.into()
}

#[cfg(not(tarpaulin_include))]
#[proc_macro]
pub fn load_image(input: TokenStream) -> TokenStream {
    let item: syn::LitInt = syn::parse(input).expect("failed to parse input");
    let num_to_gen = item.base10_parse::<usize>().unwrap();
    let filename = format!("test_{:04}.img", num_to_gen);
    let output = quote! {
        DiskFile::open(#filename).unwrap()
    };
    output.into()
}

#[cfg(not(tarpaulin_include))]
#[proc_macro]
pub fn assert_or_err(input: TokenStream) -> TokenStream {
    let params = syn::parse_macro_input!(input with syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>::parse_separated_nonempty);
    assert_eq!(params.len(), 2);
    let test = &params[0];
    let error = &params[1];
    let output = quote! {
        if (!(#test)) {
            return Err(#error.into());
        }
    };
    output.into()
}

#[cfg(not(tarpaulin_include))]
#[proc_macro_attribute]
pub fn test_fs(_: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = syn::parse_macro_input!(item as syn::ItemFn);
    //panic!("{:#?}",input_fn);
    let input_sig = input_fn.sig.clone();
    let input_blk = input_fn.block.stmts.clone();
    let output = quote! {
        #[test]
        #input_sig {
            amfs::test::logging::init_log();
            #(#input_blk)*
        }
    };
    output.into()
}
