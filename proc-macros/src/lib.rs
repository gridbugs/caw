use heck::ToUpperCamelCase;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use std::{
    collections::{HashMap, HashSet},
    mem,
};
use syn::{
    parse_macro_input,
    punctuated::Punctuated,
    token::{Comma, Plus},
    AngleBracketedGenericArguments, AssocType, Attribute, Expr, ExprLit,
    GenericArgument, GenericParam, Ident, ItemStruct, Lit, LitStr, Meta, Path,
    PathArguments, PathSegment, Token, TraitBound, TraitBoundModifier, Type,
    TypeImplTrait, TypeParam, TypeParamBound, TypePath,
};

fn convert_snake_to_camel(ident: Ident) -> Ident {
    let upper_camel_case_string =
        ident.to_string().as_str().to_upper_camel_case();
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
        ident: format_ident!(
            "__{}T",
            convert_snake_to_camel(field_ident.clone())
        ),
        colon_token: None,
        bounds: signal_type_bounds(item_type),
        eq_token: None,
        default: None,
    }
}

fn generic_type_param(
    field_ident: &Ident,
    bounds: &Punctuated<TypeParamBound, Plus>,
) -> TypeParam {
    TypeParam {
        attrs: Vec::new(),
        ident: format_ident!(
            "__{}T",
            convert_snake_to_camel(field_ident.clone())
        ),
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

/// The type `impl Signal<Item = X>`.
fn impl_signal_trait_type(item_type: Type) -> Type {
    Type::ImplTrait(TypeImplTrait {
        impl_token: Token![impl](Span::call_site()),
        bounds: signal_type_bounds(item_type),
    })
}

fn type_with_type_params(
    ident: Ident,
    args: Punctuated<GenericArgument, Comma>,
) -> Type {
    let generic_arguments = AngleBracketedGenericArguments {
        colon2_token: None,
        lt_token: Token![<](Span::call_site()),
        args,
        gt_token: Token![>](Span::call_site()),
    };
    let path_segment = PathSegment {
        ident,
        arguments: PathArguments::AngleBracketed(generic_arguments),
    };
    type_of_single_segment(path_segment)
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
pub fn signal_builder(input: TokenStream) -> TokenStream {
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
    let mut field_with_default_types_impl_signal = Vec::new();
    let mut field_default_values = Vec::new();
    let mut signal_field_with_default_bounds = Vec::new();
    let mut signal_field_with_default_idents = Vec::new();
    let mut signal_field_with_default_type_param_idents = Vec::new();
    let mut generic_field_with_default_type_param_idents = Vec::new();
    let mut signal_type_param_to_item_type = HashMap::new();
    let mut signal_type_params_with_default_values = HashSet::new();
    let mut generic_field_type_to_default_type = HashMap::new();
    let mut build_fn = Vec::new();
    let mut build_ty = Vec::new();
    let attrs = mem::replace(&mut input.attrs, Vec::new());
    for attr in attrs {
        if attr.path().is_ident("build_fn") {
            if let Some(s) = attr_lit_str(&attr) {
                let ident: Ident = s.parse().expect("Expected identifier");
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
        input.attrs.push(attr);
    }
    if build_fn.len() != build_ty.len() || build_fn.len() > 1 {
        panic!(
            "The `build_fn` and `build_ty` attributes should both be set \
            exactly once, or not set at all."
        );
    }
    for field in input.fields.iter_mut() {
        if let Some(ident) = field.ident.as_ref() {
            all_field_idents_in_order.push(ident.clone());
            let mut signal = false;
            let mut default = None;
            let mut generic = false;
            let mut generic_with_constraints: Punctuated<TypeParamBound, Plus> =
                Punctuated::new();
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
                field.attrs.push(attr);
            }
            let original_type = field.ty.clone();
            if signal {
                let signal_type_param =
                    signal_type_param(field.ty.clone(), &ident);
                signal_type_param_to_item_type
                    .insert(signal_type_param.ident.clone(), field.ty.clone());
                if default.is_some() {
                    signal_type_params_with_default_values
                        .insert(signal_type_param.ident.clone());
                    signal_field_with_default_type_param_idents
                        .push(signal_type_param.ident.clone());
                }
                field.ty = type_of_ident(signal_type_param.ident.clone());
                input
                    .generics
                    .params
                    .push(GenericParam::Type(signal_type_param));
            }
            if generic {
                let generic_type_param =
                    generic_type_param(&ident, &generic_with_constraints);
                if default.is_some() {
                    generic_field_type_to_default_type.insert(
                        generic_type_param.ident.clone(),
                        field.ty.clone(),
                    );
                    generic_field_with_default_type_param_idents
                        .push(generic_type_param.ident.clone());
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
                if signal {
                    field_with_default_types_impl_signal
                        .push(impl_signal_trait_type(original_type.clone()));
                    signal_field_with_default_idents.push(ident.clone());
                    signal_field_with_default_bounds
                        .push(signal_type_bounds(original_type));
                } else {
                    if generic {
                        generic_field_with_default_idents.push(ident.clone());
                        generic_field_with_default_types.push(field.ty.clone());
                        generic_field_with_default_constraints
                            .push(generic_with_constraints);
                    } else {
                        regular_field_with_default_idents.push(ident.clone());
                        regular_field_with_default_types.push(field.ty.clone());
                    }
                    // Not a signal field, so just copy the type again
                    // into this list. It will be used for the
                    // argument type of setter functions that don't
                    // set signal fields.
                    field_with_default_types_impl_signal.push(field.ty.clone());
                }
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
                let ty = if signal_type_params_with_default_values
                    .contains(&type_param.ident)
                {
                    // This type param has a default value, so the
                    // abstract type parameter name won't match the
                    // type of the field. Instead we need to use an
                    // `impl Signal<Item = ...>` type here, since
                    // there is no way to refer to this type by its
                    // name.
                    let item_type = signal_type_param_to_item_type
                        [&type_param.ident]
                        .clone();
                    impl_signal_trait_type(item_type)
                } else if let Some(default_type) =
                    generic_field_type_to_default_type.get(&type_param.ident)
                {
                    default_type.clone()
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
        type_with_type_params(builder_ident.clone(), args)
    };
    let setter_value_type_ident = Ident::new("__T", Span::call_site());
    let make_setter_return_type = |type_param_ident| {
        let mut args = Punctuated::new();
        for generic_param in &input.generics.params {
            if let GenericParam::Type(ref type_param) = generic_param {
                let type_ident = if &type_param.ident == type_param_ident {
                    // The current type parameter is the one that
                    // should be replaced by the argument type of
                    // the getter of the current field.
                    setter_value_type_ident.clone()
                } else {
                    type_param.ident.clone()
                };
                args.push(GenericArgument::Type(type_of_ident(type_ident)));
            }
        }
        type_with_type_params(builder_ident.clone(), args)
    };
    let signal_field_with_default_setter_return_types =
        signal_field_with_default_type_param_idents
            .iter()
            .map(make_setter_return_type)
            .collect::<Vec<_>>();
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
    let signal_field_with_default_setter_all_fields_except_current =
        signal_field_with_default_idents
            .iter()
            .map(all_fields_except_current)
            .collect::<Vec<_>>();
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
                #(#field_without_default_idents: #field_without_default_types),*
            ) -> #new_fn_return_type {
                #builder_ident {
                    #(#field_without_default_idents),*,
                    #(#field_with_default_idents: #field_default_values),*,
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

            // Generate a setter function for each field with a
            // default value. Fields without default values are set in
            // the `new` function instead.
            #(pub fn #signal_field_with_default_idents<__T>(
                self,
                #signal_field_with_default_idents: __T,
            ) -> #signal_field_with_default_setter_return_types
            where
                __T: #signal_field_with_default_bounds
            {
                let Self {
                    #signal_field_with_default_setter_all_fields_except_current,
                    ..
                } = self;
                #builder_ident {
                    #signal_field_with_default_setter_all_fields_except_current,
                    #signal_field_with_default_idents,
                }
            })*

            // Generate a setter function for each generic field with
            // a default value. Fields without default values are set
            // in the `new` function instead.
            #(fn #generic_field_with_default_idents<__T>(
                self,
                #generic_field_with_default_idents: __T,
            ) -> #generic_field_with_default_setter_return_types
            where
                __T: #generic_field_with_default_constraints
            {
                let Self {
                    #generic_field_with_default_setter_all_fields_except_current,
                    ..
                } = self;
                #builder_ident {
                    #generic_field_with_default_setter_all_fields_except_current,
                    #generic_field_with_default_idents,
                }
            })*

            // Call the user-provided `build_fn` if any. If no
            // `build_fn` was set, don't generate a `build`
            // method. This is a valid use-case, as a user may wish to
            // implement the `build` method by hand for some
            // non-trivial signal builders.
            #(fn build(self) -> #build_ty {
                let Self { #all_field_idents_in_order } = self;
                #build_fn(#all_field_idents_in_order)
            })*
        }
    };
    TokenStream::from(expanded)
}
