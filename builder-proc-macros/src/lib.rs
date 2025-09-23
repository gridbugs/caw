use heck::ToUpperCamelCase;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use std::collections::HashMap;
use syn::{
    AngleBracketedGenericArguments, Attribute, Expr, ExprLit, GenericArgument,
    GenericParam, Generics, Ident, ItemStruct, Lit, LitStr, Meta, Pat, PatRest,
    Path, PathArguments, PathSegment, Token, Type, TypeParam, TypeParamBound,
    TypePath, WhereClause, WherePredicate, parse_macro_input, parse_quote,
    punctuated::Punctuated,
    token::{Comma, Plus},
};

fn convert_snake_to_camel(ident: Ident) -> Ident {
    let upper_camel_case_string =
        ident.to_string().as_str().to_upper_camel_case();
    Ident::new(upper_camel_case_string.as_str(), ident.span())
}

fn generic_type_param(
    ident: Ident,
    bounds: &Punctuated<TypeParamBound, Plus>,
) -> TypeParam {
    TypeParam {
        attrs: Vec::new(),
        ident,
        colon_token: None,
        bounds: bounds.clone(),
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

fn bracketed_generic_arguments(
    args: Punctuated<GenericArgument, Comma>,
) -> AngleBracketedGenericArguments {
    AngleBracketedGenericArguments {
        colon2_token: None,
        lt_token: Token![<](Span::call_site()),
        args,
        gt_token: Token![>](Span::call_site()),
    }
}

fn type_with_type_params(
    ident: Ident,
    args: Punctuated<GenericArgument, Comma>,
) -> Type {
    let generic_arguments = bracketed_generic_arguments(args);
    let path_segment = PathSegment {
        ident,
        arguments: PathArguments::AngleBracketed(generic_arguments),
    };
    type_of_single_segment(path_segment)
}

fn generics_for_function_def(
    params: Punctuated<GenericParam, Comma>,
) -> Generics {
    Generics {
        lt_token: Some(Token![<](Span::call_site())),
        params,
        gt_token: Some(Token![>](Span::call_site())),
        where_clause: None,
    }
}

fn attr_lit_str(attr: &Attribute) -> Option<&LitStr> {
    if let Meta::NameValue(ref meta_name_value) = attr.meta {
        if let Expr::Lit(ExprLit {
            lit: Lit::Str(ref s),
            ..
        }) = meta_name_value.value
        {
            return Some(s);
        }
    }
    None
}

/// Pass this a struct definition to generate a constructor, setters,
/// and a build method treating the struct as a builder in the builder
/// pattern.
#[proc_macro]
pub fn builder(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as ItemStruct);
    let builder_ident = input.ident.clone();
    let mut all_field_idents_in_order: Punctuated<Ident, Comma> =
        Punctuated::new();
    let mut field_without_default_idents = Vec::new();
    let mut field_without_default_types = Vec::new();
    let mut field_with_default_idents = Vec::new();
    let mut field_with_default_types = Vec::new();
    let mut regular_field_with_default_idents = Vec::new();
    let mut regular_field_with_default_types = Vec::new();
    let mut generic_field_with_default_idents = Vec::new();
    let mut generic_field_with_default_types = Vec::new();
    let mut generic_field_with_default_constraints = Vec::new();
    let mut generic_field_with_default_extra_where_predicates = Vec::new();
    let mut field_default_values = Vec::new();
    let mut generic_field_with_default_type_param_idents = Vec::new();
    let mut generic_field_type_to_default_type = HashMap::new();
    let mut build_fn = Vec::new();
    let mut build_ty = Vec::new();
    let mut constructor = None;
    let mut generic_setter_type_name = None;
    let mut constructor_doc = None;
    let mut constructor_where_predicates = Vec::new();
    let attrs = std::mem::take(&mut input.attrs);
    for attr in attrs {
        if attr.path().is_ident("constructor") {
            if let Some(s) = attr_lit_str(&attr) {
                let ident: Ident = s.parse().expect("Expected identifier");
                constructor = Some(ident);
            }
            continue;
        }
        if attr.path().is_ident("constructor_doc") {
            if let Some(s) = attr_lit_str(&attr) {
                constructor_doc = Some(s.value());
            }
            continue;
        }
        if attr.path().is_ident("constructor_where") {
            if let Some(s) = attr_lit_str(&attr) {
                let where_predicates: WherePredicate =
                    s.parse().expect("Expected where predicates");
                constructor_where_predicates.push(where_predicates);
            }
            continue;
        }
        if attr.path().is_ident("build_fn") {
            if let Some(s) = attr_lit_str(&attr) {
                let ident: Path = s.parse().expect("Expected identifier");
                build_fn.push(ident);
            }
            continue;
        }
        if attr.path().is_ident("build_ty") {
            if let Some(s) = attr_lit_str(&attr) {
                let ty: Type = s.parse().expect("Expected type");
                build_ty.push(ty);
            }
            continue;
        }
        if attr.path().is_ident("extra_generic") {
            let list = attr.meta.require_list().expect("Expected list");
            let args = list
                .parse_args_with(
                    Punctuated::<LitStr, Token![,]>::parse_terminated,
                )
                .expect("Failed to parse argument list");
            assert!(
                args.len() == 2,
                "#[extra_generics(type, constraint)] expected 2 arguments, got {}.",
                args.len()
            );
            let type_ident = args.get(0).unwrap();
            let constraint = args.get(1).unwrap();
            let type_ident: Ident =
                type_ident.parse().expect("Expected identifier");
            let constraint = constraint
                .parse_with(
                    Punctuated::<TypeParamBound, Token![+]>::parse_terminated,
                )
                .expect("Expected constraint");
            let generic_type_param =
                generic_type_param(type_ident, &constraint);
            input
                .generics
                .params
                .push(GenericParam::Type(generic_type_param));
            continue;
        }
        if attr.path().is_ident("generic_setter_type_name") {
            if let Some(s) = attr_lit_str(&attr) {
                let ident: Ident = s.parse().expect("Expected identifier");
                generic_setter_type_name = Some(ident);
            }
            continue;
        }
        input.attrs.push(attr);
    }
    let generic_setter_type_name = generic_setter_type_name
        .unwrap_or_else(|| Ident::new("__T", Span::call_site()));
    let constructor = constructor.expect("Missing \"constructor\" attribute");
    let constructor_doc = format!(
        " {}",
        constructor_doc.unwrap_or_else(|| format!(
            "Shorthand for [`{}::new`].",
            builder_ident
        ))
    );
    let constructor_doc: Attribute = parse_quote!(#[doc = #constructor_doc]);
    let constructor_where_clause = WhereClause {
        where_token: Token![where](Span::call_site()),
        predicates: {
            let mut predicates = Punctuated::new();
            for where_predicate in constructor_where_predicates {
                predicates.push(where_predicate);
            }
            predicates
        },
    };
    if build_fn.len() != build_ty.len() || build_fn.len() > 1 {
        panic!(
            "The `build_fn` and `build_ty` attributes should both be set \
            exactly once, or not set at all."
        );
    }
    let mut constructor_generics = input.generics.params.clone();
    for field in input.fields.iter_mut() {
        if let Some(ident) = field.ident.as_ref() {
            all_field_idents_in_order.push(ident.clone());
            let mut default = None;
            let mut generic = false;
            let mut generic_with_constraints: Punctuated<TypeParamBound, Plus> =
                Punctuated::new();
            let mut extra_where_predicates: Punctuated<WherePredicate, Comma> =
                Punctuated::new();
            let mut generic_name = None;
            let attrs = std::mem::take(&mut field.attrs);
            for attr in attrs {
                if attr.path().is_ident("default") {
                    if let Meta::NameValue(ref meta_name_value) = attr.meta {
                        default = Some(meta_name_value.value.clone());
                        continue;
                    }
                }
                if attr.path().is_ident("generic") {
                    generic = true;
                    continue;
                }
                if attr.path().is_ident("generic_with_constraint") {
                    generic = true;
                    if let Some(s) = attr_lit_str(&attr) {
                        let constraint: TypeParamBound =
                            s.parse().expect("Expected constraint");
                        generic_with_constraints.push(constraint);
                    }
                    continue;
                }
                if attr.path().is_ident("generic_name") {
                    generic = true;
                    if let Some(s) = attr_lit_str(&attr) {
                        let name: Ident =
                            s.parse().expect("Expected identifier");
                        generic_name = Some(name);
                    }
                    continue;
                }
                if attr.path().is_ident("extra_where_predicate") {
                    generic = true;
                    if let Some(s) = attr_lit_str(&attr) {
                        let where_predicate: WherePredicate =
                            s.parse().expect("Expected where predicates");
                        extra_where_predicates.push(where_predicate);
                    }
                    continue;
                }
                field.attrs.push(attr);
            }
            if generic {
                let generic_ident = generic_name.unwrap_or_else(|| {
                    format_ident!(
                        "__{}T",
                        convert_snake_to_camel(ident.clone())
                    )
                });
                let generic_type_param = generic_type_param(
                    generic_ident,
                    &generic_with_constraints,
                );
                if default.is_some() {
                    generic_field_type_to_default_type.insert(
                        generic_type_param.ident.clone(),
                        field.ty.clone(),
                    );
                    generic_field_with_default_type_param_idents
                        .push(generic_type_param.ident.clone());
                } else {
                    constructor_generics
                        .push(GenericParam::Type(generic_type_param.clone()));
                }
                field.ty = type_of_ident(generic_type_param.ident.clone());
                input
                    .generics
                    .params
                    .push(GenericParam::Type(generic_type_param));
            }
            if let Some(default) = default {
                field_with_default_idents.push(ident.clone());
                field_with_default_types.push(field.ty.clone());
                if generic {
                    generic_field_with_default_idents.push(ident.clone());
                    generic_field_with_default_types.push(field.ty.clone());
                    generic_field_with_default_constraints
                        .push(generic_with_constraints);
                    generic_field_with_default_extra_where_predicates
                        .push(extra_where_predicates);
                } else {
                    regular_field_with_default_idents.push(ident.clone());
                    regular_field_with_default_types.push(field.ty.clone());
                }
                field_default_values.push(default);
            } else {
                field_without_default_idents.push(ident.clone());
                field_without_default_types.push(field.ty.clone());
            }
        }
    }
    let constructor_generics = generics_for_function_def(constructor_generics);
    let new_fn_return_type_generics = {
        let mut args = Punctuated::new();
        for generic_param in &input.generics.params {
            if let GenericParam::Type(type_param) = generic_param {
                let ty = if let Some(default_type) =
                    generic_field_type_to_default_type.get(&type_param.ident)
                {
                    // This type param has a default value, so the
                    // abstract type parameter name won't match the
                    // type of the field. Use the type from the
                    // original struct definition as the default type
                    // of the field.
                    default_type.clone()
                } else {
                    // No default value, so `new` will be passed a
                    // value. This means we can use the name of the
                    // type parameter as this type argument. This case
                    // is also hit for any non-generic type
                    // parameters.
                    type_of_ident(type_param.ident.clone())
                };
                args.push(GenericArgument::Type(ty));
            }
        }
        bracketed_generic_arguments(args)
    };
    // The return type of the `Builder::new` method.
    let new_fn_return_type = {
        let path_segment = PathSegment {
            ident: builder_ident.clone(),
            arguments: PathArguments::AngleBracketed(
                new_fn_return_type_generics.clone(),
            ),
        };
        type_of_single_segment(path_segment)
    };
    let make_setter_return_type = |type_param_ident| {
        let mut args = Punctuated::new();
        for generic_param in &input.generics.params {
            if let GenericParam::Type(type_param) = generic_param {
                let type_ident = if &type_param.ident == type_param_ident {
                    // The current type parameter is the one that
                    // should be replaced by the argument type of
                    // the getter of the current field.
                    generic_setter_type_name.clone()
                } else {
                    type_param.ident.clone()
                };
                args.push(GenericArgument::Type(type_of_ident(type_ident)));
            }
        }
        type_with_type_params(builder_ident.clone(), args)
    };
    let generic_field_with_default_setter_return_types =
        generic_field_with_default_type_param_idents
            .iter()
            .map(make_setter_return_type)
            .collect::<Vec<_>>();
    let all_fields_except_current = |current_field_ident: &Ident| {
        let mut other_fields: Punctuated<Ident, Comma> = Punctuated::new();
        for field in input.fields.iter() {
            if let Some(ref field_ident) = field.ident {
                if field_ident != current_field_ident {
                    other_fields.push(field_ident.clone());
                }
            }
        }
        other_fields
    };
    let regular_field_with_default_setter_all_fields_except_current =
        regular_field_with_default_idents
            .iter()
            .map(all_fields_except_current)
            .collect::<Vec<_>>();
    let generic_field_with_default_setter_all_fields_except_current =
        generic_field_with_default_idents
            .iter()
            .map(all_fields_except_current)
            .collect::<Vec<_>>();
    let generic_field_with_default_setter_destructure_self =
        generic_field_with_default_idents
            .iter()
            .map(|current| {
                let fields = all_fields_except_current(current);
                let pat: Pat = parse_quote! {
                    Self { #fields }
                };
                let mut pat_struct = if let Pat::Struct(pat_struct) = pat {
                    pat_struct
                } else {
                    panic!("unexpected result of parsing fields");
                };
                pat_struct.rest = Some(PatRest {
                    attrs: Vec::new(),
                    dot2_token: Token![..](Span::call_site()),
                });
                pat_struct
            })
            .collect::<Vec<_>>();
    let (impl_generics, ty_generics, where_clause) =
        input.generics.split_for_impl();
    let expanded = quote! {
        #input

        impl #impl_generics #builder_ident #ty_generics #where_clause {

            // Create a new builder with default values set for some
            // fields, and with other fields set by arguments to this
            // method. Note that the return type is not `Self`, as the
            // type parameters of fields with default values are
            // concrete (whatever the type of the default value for
            // the field is) raher than abstract.
            pub fn new(
                #(#field_without_default_idents: #field_without_default_types,)*
            ) -> #new_fn_return_type {
                #builder_ident {
                    #(#field_without_default_idents,)*
                    #(#field_with_default_idents: #field_default_values,)*
                }
            }

            // Generate a setter function for each regular field with
            // a default value. Fields without default values are set
            // in the `new` function instead.
            #(pub fn #regular_field_with_default_idents(
                    self,
                    #regular_field_with_default_idents: #regular_field_with_default_types,
            ) -> Self {
                let Self {
                    #regular_field_with_default_setter_all_fields_except_current,
                    ..
                } = self;
                #builder_ident {
                    #regular_field_with_default_setter_all_fields_except_current,
                    #regular_field_with_default_idents,
                }
            })*

            // Generate a setter function for each generic field with
            // a default value. Fields without default values are set
            // in the `new` function instead.
            #(pub fn #generic_field_with_default_idents<#generic_setter_type_name>(
                self,
                #generic_field_with_default_idents: #generic_setter_type_name,
            ) -> #generic_field_with_default_setter_return_types
            where
                #generic_setter_type_name: #generic_field_with_default_constraints,
                #generic_field_with_default_extra_where_predicates
            {
                let #generic_field_with_default_setter_destructure_self = self;
                #builder_ident {
                    #generic_field_with_default_idents,
                    #generic_field_with_default_setter_all_fields_except_current
                }
            })*

            // Call the user-provided `build_fn` if any. If no
            // `build_fn` was set, don't generate a `build`
            // method. This is a valid use-case, as a user may wish to
            // implement the `build` method by hand for some
            // non-trivial builders.
            #(pub fn build(self) -> #build_ty {
                let Self { #all_field_idents_in_order } = self;
                #build_fn(#all_field_idents_in_order)
            })*
        }

        #constructor_doc
        pub fn #constructor #constructor_generics (
            #(#field_without_default_idents: #field_without_default_types,)*
        ) -> #new_fn_return_type
            #constructor_where_clause
        {
            #builder_ident::#new_fn_return_type_generics::new(#(#field_without_default_idents,)*)
        }
    };
    TokenStream::from(expanded)
}
