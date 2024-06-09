use heck::ToUpperCamelCase;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use std::{
    collections::{HashMap, HashSet},
    mem,
};
use syn::{
    parse_macro_input, punctuated::Punctuated, token::Plus, AngleBracketedGenericArguments,
    AssocType, GenericArgument, GenericParam, Ident, ItemStruct, Meta, Path, PathArguments,
    PathSegment, Token, TraitBound, TraitBoundModifier, Type, TypeImplTrait, TypeParam,
    TypeParamBound, TypePath,
};

fn convert_snake_to_camel(ident: Ident) -> Ident {
    let upper_camel_case_string = ident.to_string().as_str().to_upper_camel_case();
    Ident::new(upper_camel_case_string.as_str(), ident.span())
}

/// The type bounds `Signal<Item = X>` for a given `X`.
fn signal_type_bounds(item_type: Type) -> Punctuated<TypeParamBound, Plus> {
    // The `Item = X` component of the output
    let item_assoc_type = GenericArgument::AssocType(AssocType {
        ident: Ident::new("Item", Span::call_site()),
        generics: None,
        eq_token: Token![=](Span::call_site()),
        ty: item_type,
    });
    let mut args = Punctuated::new();
    args.push(item_assoc_type);
    let generic_arguments = AngleBracketedGenericArguments {
        colon2_token: None,
        lt_token: Token![<](Span::call_site()),
        args,
        gt_token: Token![>](Span::call_site()),
    };
    let signal_trait_segment = PathSegment {
        ident: Ident::new("Signal", Span::call_site()),
        arguments: PathArguments::AngleBracketed(generic_arguments),
    };
    let mut segments = Punctuated::new();
    segments.push(signal_trait_segment);
    let bound = TypeParamBound::Trait(TraitBound {
        paren_token: None,
        modifier: TraitBoundModifier::None,
        lifetimes: None,
        path: Path {
            leading_colon: None,
            segments,
        },
    });
    let mut bounds = Punctuated::new();
    bounds.push(bound);
    bounds
}

/// Returns a representation of `T: Signal<Item = X>`.
fn signal_type_param(item_type: Type, field_ident: &Ident) -> TypeParam {
    TypeParam {
        attrs: Vec::new(),
        ident: format_ident!("__{}T", convert_snake_to_camel(field_ident.clone())),
        colon_token: None,
        bounds: signal_type_bounds(item_type),
        eq_token: None,
        default: None,
    }
}

fn type_of_single_segment(path_segment: PathSegment) -> Type {
    let mut segments = Punctuated::new();
    segments.push(path_segment);
    Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments,
        },
    })
}

/// A type represented by a single identifier.
fn type_of_ident(ident: Ident) -> Type {
    type_of_single_segment(PathSegment {
        ident,
        arguments: PathArguments::None,
    })
}

/// The type `impl Signal<Item = X>`.
fn impl_signal_trait_type(item_type: Type) -> Type {
    Type::ImplTrait(TypeImplTrait {
        impl_token: Token![impl](Span::call_site()),
        bounds: signal_type_bounds(item_type),
    })
}

#[proc_macro]
pub fn signal_builder(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as ItemStruct);
    let builder_ident = input.ident.clone();
    let mut field_without_default_idents = Vec::new();
    let mut field_without_default_types = Vec::new();
    let mut field_with_default_idents = Vec::new();
    let mut field_with_default_types = Vec::new();
    let mut field_default_values = Vec::new();
    let mut signal_type_param_to_item_type = HashMap::new();
    let mut signal_type_params_with_default_values = HashSet::new();
    for field in input.fields.iter_mut() {
        if let Some(ident) = field.ident.as_ref() {
            let mut signal = false;
            let mut default = None;
            let attrs = mem::replace(&mut field.attrs, Vec::new());
            for attr in attrs {
                if attr.path().is_ident("signal") {
                    signal = true;
                    continue;
                }
                if attr.path().is_ident("default") {
                    if let Meta::NameValue(ref meta_name_value) = attr.meta {
                        default = Some(meta_name_value.value.clone());
                        continue;
                    }
                }
                field.attrs.push(attr);
            }
            if signal {
                let signal_type_param = signal_type_param(field.ty.clone(), &ident);
                signal_type_param_to_item_type
                    .insert(signal_type_param.ident.clone(), field.ty.clone());
                if default.is_some() {
                    signal_type_params_with_default_values.insert(signal_type_param.ident.clone());
                }
                field.ty = type_of_ident(signal_type_param.ident.clone());

                input
                    .generics
                    .params
                    .push(GenericParam::Type(signal_type_param));
            }
            if let Some(default) = default {
                field_with_default_idents.push(ident.clone());
                field_with_default_types.push(field.ty.clone());
                field_default_values.push(default);
            } else {
                field_without_default_idents.push(ident.clone());
                field_without_default_types.push(field.ty.clone());
            }
        }
    }
    // The return type of the `Builder::new` method.
    let new_fn_return_type = {
        let mut args = Punctuated::new();
        for generic_param in &input.generics.params {
            if let GenericParam::Type(ref type_param) = generic_param {
                let ty = if signal_type_params_with_default_values.contains(&type_param.ident) {
                    // This type param has a default value, so the
                    // abstract type parameter name won't match the
                    // type of the field. Instead we need to use an
                    // `impl Signal<Item = ...>` type here, since
                    // there is no way to refer to this type by its
                    // name.
                    let item_type = signal_type_param_to_item_type[&type_param.ident].clone();
                    impl_signal_trait_type(item_type)
                } else {
                    // No default value, so `new` will be passed a
                    // value. This means we can use the name of the
                    // type parameter as this type argument. This case
                    // is also hit for any non-signal type
                    // parameters. Default values for non-signal type
                    // parameters is not supported so it's always safe
                    // to use the type parameter name in the output.
                    type_of_ident(type_param.ident.clone())
                };
                args.push(GenericArgument::Type(ty));
            }
        }
        let generic_arguments = AngleBracketedGenericArguments {
            colon2_token: None,
            lt_token: Token![<](Span::call_site()),
            args,
            gt_token: Token![>](Span::call_site()),
        };
        let path_segment = PathSegment {
            ident: builder_ident.clone(),
            arguments: PathArguments::AngleBracketed(generic_arguments),
        };
        type_of_single_segment(path_segment)
    };
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let expanded = quote! {
        #input

        impl #impl_generics #builder_ident #ty_generics #where_clause {
            pub fn new(#(#field_without_default_idents: #field_without_default_types),*)
                -> #new_fn_return_type
            {
                #builder_ident {
                    #(#field_without_default_idents),*,
                    #(#field_with_default_idents: #field_default_values),*,
                }
            }
        }
    };
    TokenStream::from(expanded)
}
