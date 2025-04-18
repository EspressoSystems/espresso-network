// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Macros for use in testing.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream, Result},
    parse_macro_input,
    punctuated::Punctuated,
    Expr, ExprArray, ExprPath, ExprTuple, Ident, LitBool, PathArguments, Token, TypePath,
};

/// Bracketed types, e.g. [A, B, `C<D>`]
/// These types can have generic parameters, whereas ExprArray items must be Expr.
#[derive(derive_builder::Builder, Debug, Clone)]
struct TypePathBracketedArray {
    /// elems
    pub elems: Punctuated<TypePath, Token![,]>,
}

/// description of a crosstest
#[derive(derive_builder::Builder, Debug, Clone)]
struct CrossTestData {
    /// implementations
    impls: ExprArray,

    /// builder impl
    #[builder(default = "syn::parse_str(\"[SimpleBuilderImplementation]\").unwrap()")]
    builder_impls: ExprArray,

    /// versions
    versions: ExprArray,

    /// types
    types: TypePathBracketedArray,

    /// name of the test
    test_name: Ident,

    /// test description/spec
    metadata: Expr,

    /// whether or not to ignore
    ignore: LitBool,
}

impl CrossTestDataBuilder {
    /// if we've extracted all the metadata
    fn is_ready(&self) -> bool {
        self.impls.is_some()
            && self.types.is_some()
            && self.versions.is_some()
            && self.test_name.is_some()
            && self.metadata.is_some()
            && self.test_name.is_some()
            && self.ignore.is_some()
    }
}

/// requisite data to generate a single test
#[derive(derive_builder::Builder, Debug, Clone)]
struct TestData {
    /// type
    ty: TypePath,

    /// impl
    imply: ExprPath,

    /// builder implementation
    builder_impl: ExprPath,

    /// impl
    version: ExprPath,

    /// name of test
    test_name: Ident,

    /// test description
    metadata: Expr,

    /// whether or not to ignore the test
    ignore: LitBool,
}

/// trait make a string lower and snake case
trait ToLowerSnakeStr {
    /// make a lower and snake case string
    fn to_lower_snake_str(&self) -> String;
}

impl ToLowerSnakeStr for ExprPath {
    fn to_lower_snake_str(&self) -> String {
        self.path
            .segments
            .iter()
            .fold(String::new(), |mut acc, s| {
                acc.push_str(&s.ident.to_string().to_lowercase());
                acc.push('_');
                acc
            })
            .to_lowercase()
    }
}

impl ToLowerSnakeStr for syn::GenericArgument {
    /// allow panic because this is a compiler error
    #[allow(clippy::panic)]
    fn to_lower_snake_str(&self) -> String {
        match self {
            syn::GenericArgument::Lifetime(l) => l.ident.to_string().to_lowercase(),
            syn::GenericArgument::Type(t) => match t {
                syn::Type::Path(p) => p.to_lower_snake_str(),
                _ => {
                    panic!("Unexpected type for GenericArgument::Type: {t:?}");
                },
            },
            syn::GenericArgument::Const(c) => match c {
                syn::Expr::Lit(l) => match &l.lit {
                    syn::Lit::Str(v) => format!("{}_", v.value().to_lowercase()),
                    syn::Lit::Int(v) => format!("{}_", v.base10_digits()),
                    _ => {
                        panic!("Unexpected type for GenericArgument::Const::Lit: {l:?}");
                    },
                },
                _ => {
                    panic!("Unexpected type for GenericArgument::Const: {c:?}");
                },
            },
            _ => {
                panic!("Unexpected type for GenericArgument: {self:?}");
            },
        }
    }
}

impl ToLowerSnakeStr for TypePath {
    fn to_lower_snake_str(&self) -> String {
        self.path
            .segments
            .iter()
            .fold(String::new(), |mut acc, s| {
                acc.push_str(&s.ident.to_string().to_lowercase());
                if let PathArguments::AngleBracketed(a) = &s.arguments {
                    acc.push('_');
                    for arg in &a.args {
                        acc.push_str(&arg.to_lower_snake_str());
                    }
                }

                acc.push('_');
                acc
            })
            .to_lowercase()
    }
}

impl ToLowerSnakeStr for ExprTuple {
    /// allow panic because this is a compiler error
    #[allow(clippy::panic)]
    fn to_lower_snake_str(&self) -> String {
        self.elems
            .iter()
            .map(|x| {
                let Expr::Path(expr_path) = x else {
                    panic!("Expected path expr, got {x:?}");
                };
                expr_path
            })
            .fold(String::new(), |mut acc, s| {
                acc.push_str(&s.to_lower_snake_str());
                acc
            })
    }
}

impl TestData {
    /// generate the code for a single test
    fn generate_test(&self) -> TokenStream2 {
        let TestData {
            ty,
            imply,
            version,
            test_name,
            metadata,
            ignore,
            builder_impl,
        } = self;

        let slow_attribute = if ignore.value() {
            // quote! { #[cfg(feature = "slow-tests")] }
            quote! { #[ignore] }
        } else {
            quote! {}
        };
        quote! {
            #[cfg(test)]
            #slow_attribute
            #[tokio::test(flavor = "multi_thread")]
            #[tracing::instrument]
            async fn #test_name() {
                hotshot::helpers::initialize_logging();

                hotshot_testing::test_builder::TestDescription::<#ty, #imply, #version>::gen_launcher((#metadata)).launch().run_test::<#builder_impl>().await;
            }
        }
    }
}

/// macro specific custom keywords
mod keywords {
    syn::custom_keyword!(Metadata);
    syn::custom_keyword!(Ignore);
    syn::custom_keyword!(TestName);
    syn::custom_keyword!(Types);
    syn::custom_keyword!(Impls);
    syn::custom_keyword!(BuilderImpls);
    syn::custom_keyword!(Versions);
}

impl Parse for TypePathBracketedArray {
    /// allow panic because this is a compiler error
    #[allow(clippy::panic)]
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let content;
        syn::bracketed!(content in input);
        let mut elems = Punctuated::new();

        while !content.is_empty() {
            let first: TypePath = content.parse()?;
            elems.push_value(first);
            if content.is_empty() {
                break;
            }
            let punct = content.parse()?;
            elems.push_punct(punct);
        }

        Ok(Self { elems })
    }
}

impl Parse for CrossTestData {
    /// allow panic because this is a compiler error
    #[allow(clippy::panic)]
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut description = CrossTestDataBuilder::create_empty();

        while !description.is_ready() {
            if input.peek(keywords::Types) {
                let _ = input.parse::<keywords::Types>()?;
                input.parse::<Token![:]>()?;
                let types = input.parse::<TypePathBracketedArray>()?; //ExprArray>()?;
                description.types(types);
            } else if input.peek(keywords::Impls) {
                let _ = input.parse::<keywords::Impls>()?;
                input.parse::<Token![:]>()?;
                let impls = input.parse::<ExprArray>()?;
                description.impls(impls);
            } else if input.peek(keywords::BuilderImpls) {
                let _ = input.parse::<keywords::BuilderImpls>()?;
                input.parse::<Token![:]>()?;
                let impls = input.parse::<ExprArray>()?;
                description.builder_impls(impls);
            } else if input.peek(keywords::Versions) {
                let _ = input.parse::<keywords::Versions>()?;
                input.parse::<Token![:]>()?;
                let versions = input.parse::<ExprArray>()?;
                description.versions(versions);
            } else if input.peek(keywords::TestName) {
                let _ = input.parse::<keywords::TestName>()?;
                input.parse::<Token![:]>()?;
                let test_name = input.parse::<Ident>()?;
                description.test_name(test_name);
            } else if input.peek(keywords::Metadata) {
                let _ = input.parse::<keywords::Metadata>()?;
                input.parse::<Token![:]>()?;
                let metadata = input.parse::<Expr>()?;
                description.metadata(metadata);
            } else if input.peek(keywords::Ignore) {
                let _ = input.parse::<keywords::Ignore>()?;
                input.parse::<Token![:]>()?;
                let ignore = input.parse::<LitBool>()?;
                description.ignore(ignore);
            } else {
                panic!(
                    "Unexpected token. Expected one of: Metadata, Ignore, Impls, BuilderImpls, Versions, Types, Testname"
                );
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        description
            .build()
            .map_err(|e| syn::Error::new(proc_macro2::Span::call_site(), format!("{e}")))
    }
}

/// Helper function to do the actual code gen
/// allow panic because this is a compiler error
#[allow(clippy::panic)]
fn cross_tests_internal(test_spec: CrossTestData) -> TokenStream {
    let impls = test_spec.impls.elems.iter().map(|t| {
        let Expr::Path(p) = t else {
            panic!("Expected Path for Impl! Got {t:?}");
        };
        p
    });

    let types = test_spec.types.elems.iter();

    let versions = test_spec.versions.elems.iter().map(|t| {
        let Expr::Path(p) = t else {
            panic!("Expected Path for Version! Got {t:?}");
        };
        p
    });

    let builder_impls = test_spec.builder_impls.elems.iter().map(|t| {
        let Expr::Path(p) = t else {
            panic!("Expected Path for BuilderImpl! Got {t:?}");
        };
        p
    });

    let mut result = quote! {};
    for ty in types.clone() {
        let mut type_mod = quote! {};
        for imp in impls.clone() {
            for builder_impl in builder_impls.clone() {
                for version in versions.clone() {
                    let test_data = TestDataBuilder::create_empty()
                        .test_name(test_spec.test_name.clone())
                        .metadata(test_spec.metadata.clone())
                        .ignore(test_spec.ignore.clone())
                        .version(version.clone())
                        .imply(imp.clone())
                        .builder_impl(builder_impl.clone())
                        .ty(ty.clone())
                        .build()
                        .unwrap();
                    let test = test_data.generate_test();

                    let impl_str = format_ident!("{}", imp.to_lower_snake_str());
                    let impl_result = quote! {
                        pub mod #impl_str {
                            use super::*;
                            #test
                        }
                    };
                    type_mod.extend(impl_result);
                }
            }
        }
        let ty_str = format_ident!("{}", ty.to_lower_snake_str());
        let typ_result = quote! {
            pub mod #ty_str {
                use super::*;
                #type_mod
            }
        };
        result.extend(typ_result);
    }
    let name = test_spec.test_name;
    quote! {
        pub mod #name {
            use super::*;
            #result
        }
    }
    .into()
}

/// Generate a cartesian product of tests across all types
/// Arguments:
/// - `Impls: []` - a list of types that implement nodetype
/// - `Metadata`: `TestMetadata::default()` - test metadata
/// - `Types: []` - a list types that implement `NodeImplementation` over the types in `Impls`
/// - `TestName: example_test` - the name of the test
/// - `Ignore`: whether or not this set of tests are ignored
///   Example usage: see tests in this module
#[proc_macro]
pub fn cross_tests(input: TokenStream) -> TokenStream {
    let test_spec = parse_macro_input!(input as CrossTestData);
    cross_tests_internal(test_spec)
}

/// Macro to test multiple `TaskState` scripts at once.
///
/// Usage:
///
///   `test_scripts[inputs, script1, script2, ...]`
///
/// The generated test will:
///   - take the first entry of `inputs`, which should be a `Vec<HotShotEvent<TYPES>>`,
///   - feed this to each task in order, validating any output received before moving on to the next one,
///   - repeat the previous steps, with the aggregated outputs received from all tasks used as the new input set,
///   - repeat until no more output has been generated by any task, and finally
///   - proceed to the next entry of inputs.
///
/// # Panics
///
/// The macro panics if the input stream cannot be parsed.
/// The test will panic if the any of the scripts has a different number of stages from the input.
#[allow(clippy::too_many_lines)]
#[proc_macro]
pub fn test_scripts(input: proc_macro::TokenStream) -> TokenStream {
    // Parse the input as an iter of Expr
    let inputs: Vec<_> = syn::parse::Parser::parse2(
        syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated,
        input.into(),
    )
    .unwrap()
    .into_iter()
    .collect();

    let test_inputs = &inputs[0];

    let test_inputs_name = quote::quote!(#test_inputs).to_string();

    let scripts = &inputs[1..];

    let output_index_names: Vec<_> = scripts
        .iter()
        .map(|i| format_ident!("{}_output_index", quote::quote!(#i).to_string()))
        .collect();

    let task_expectations: Vec<_> = scripts
        .iter()
        .map(|i| format_ident!("{}_expectations", quote::quote!(#i).to_string()))
        .collect();

    let script_names: Vec<_> = scripts
        .iter()
        .map(|i| quote::quote!(#i).to_string())
        .collect();

    let expanded = quote! { {

    use hotshot_testing::script::{
        panic_extra_output_in_script, panic_missing_output_in_script, validate_output_or_panic_in_script,
        validate_task_state_or_panic_in_script,
    };

    use hotshot_testing::{predicates::{Predicate, PredicateResult}};
    use async_broadcast::broadcast;
    use hotshot_task_impls::events::HotShotEvent;
    use tokio::time::timeout;
    use hotshot_task::task::{Task, TaskState};
    use hotshot_types::traits::node_implementation::NodeType;
    use std::sync::Arc;

    async {

    let (to_task, mut from_test) = broadcast(1024);
    let (to_test, mut from_task) = broadcast(1024);

    let mut loop_receiver = from_task.clone();

    #(let mut #task_expectations = #scripts.expectations;)*

    #(assert!(#task_expectations.len() == #test_inputs.len(), "Number of stages in {} does not match the number of stages in {}", #script_names, #test_inputs_name);)*

    for (stage_number, input_group) in #test_inputs.into_iter().enumerate() {

    #(let mut #output_index_names = 0;)*

        for input in &input_group {
            #(
                tracing::debug!("Test sent: {:?}", input);

                to_task
                    .broadcast(input.clone().into())
                    .await
                    .expect("Failed to broadcast input message");


                let _ = #scripts.state
                    .handle_event(input.clone().into(), &to_test, &from_test)
                    .await
                    .inspect_err(|e| tracing::info!("{e}"));

                while from_test.try_recv().is_ok() {}

                let mut result = PredicateResult::Incomplete;

                while let Ok(Ok(received_output)) = timeout(#scripts.timeout, from_task.recv_direct()).await {
                    tracing::debug!("Test received: {:?}", received_output);

                    let output_asserts = &mut #task_expectations[stage_number].output_asserts;

                    if #output_index_names >= output_asserts.len() {
                            panic_extra_output_in_script(stage_number, #script_names.to_string(), &received_output);
                    };

                    let assert = &mut output_asserts[#output_index_names];

                    result = validate_output_or_panic_in_script(
                        stage_number,
                        #script_names.to_string(),
                        &received_output,
                        &**assert
                    )
                    .await;

                    if result == PredicateResult::Pass {
                        #output_index_names += 1;
                    }
                }
            )*
        }

        while let Ok(input) = loop_receiver.try_recv() {
            #(
                tracing::debug!("Test sent: {:?}", input);

                to_task
                    .broadcast(input.clone().into())
                    .await
                    .expect("Failed to broadcast input message");

                let _ = #scripts.state
                    .handle_event(input.clone().into(), &to_test, &from_test)
                    .await
                    .inspect_err(|e| tracing::info!("{e}"));

                while from_test.try_recv().is_ok() {}

                let mut result = PredicateResult::Incomplete;
                while let Ok(Ok(received_output)) = timeout(#scripts.timeout, from_task.recv_direct()).await {
                    tracing::debug!("Test received: {:?}", received_output);

                    let output_asserts = &mut #task_expectations[stage_number].output_asserts;

                    if #output_index_names >= output_asserts.len() {
                            panic_extra_output_in_script(stage_number, #script_names.to_string(), &received_output);
                    };

                    let mut assert = &mut output_asserts[#output_index_names];

                    result = validate_output_or_panic_in_script(
                        stage_number,
                        #script_names.to_string(),
                        &received_output,
                        &**assert
                    )
                    .await;

                    if result == PredicateResult::Pass {
                        #output_index_names += 1;
                    }
                }
            )*
        }

        #(
            let output_asserts = &#task_expectations[stage_number].output_asserts;

            if #output_index_names < output_asserts.len() {
                panic_missing_output_in_script(stage_number, #script_names.to_string(), &output_asserts[#output_index_names]);
            }

            let task_state_asserts = &mut #task_expectations[stage_number].task_state_asserts;

            for assert in task_state_asserts {
                validate_task_state_or_panic_in_script(stage_number, #script_names.to_string(), &#scripts.state, &**assert).await;
            }
        )*
    } }

    }

    };

    expanded.into()
}

/// Macro to run the test suite with `TaskState` scripts at once with the ability to.
/// Randomize the input values using a consistent seed value.
///
/// **Note** When using `random!` you should use `all_predicates` in the output to ensure
/// that the test does not fail due to events happening out of order!
///
/// Usage:
///
///   `run_test[inputs, script1, script2, ...]`
///
/// The generated test will:
///   - take the first entry of `inputs`, which should be a `Vec<HotShotEvent<TYPES>>`,
///   - if the input is random, it'll generate a `seed` and shuffle the inputs.
///   - if the input is serial, it will execute in order.
///   - print the seed being used.
///   - feed this to each task in order, validating any output received before moving on to the next one,
///   - repeat the previous steps, with the aggregated outputs received from all tasks used as the new input set,
///   - repeat until no more output has been generated by any task, and finally
///   - proceed to the next entry of inputs.
///   - print the seed being used (again) to make finding it a bit easier.
///
/// # Panics
///
/// The macro panics if the input stream cannot be parsed.
/// The test will panic if the any of the scripts has a different number of stages from the input.
#[proc_macro]
pub fn run_test(input: TokenStream) -> TokenStream {
    // Parse the input as an iter of Expr
    let inputs: Vec<_> = syn::parse::Parser::parse2(
        syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated,
        input.into(),
    )
    .unwrap()
    .into_iter()
    .collect();

    // Separate the first input (which should be the InputOrder enum)
    let test_inputs = &inputs[0];
    let scripts = &inputs[1..];

    // Generate code for shuffling and flattening inputs
    let expanded = quote! {
        {
            use rand::{
                SeedableRng, rngs::StdRng,
                seq::SliceRandom
            };
            use hotshot_task_impls::events::HotShotEvent;
            use hotshot_task::task::TaskState;
            use hotshot_types::traits::node_implementation::NodeType;
            use hotshot_testing::script::InputOrder;

            async {
                let seed: u64 = rand::random();
                tracing::info!("Running test with seed {seed}");
                let mut rng = StdRng::seed_from_u64(seed);
                let mut shuffled_inputs = Vec::new();

                for (stage_number, input_order) in #test_inputs.into_iter().enumerate() {
                    match input_order {
                        InputOrder::Random(mut events) => {
                            events.shuffle(&mut rng);
                            shuffled_inputs.push(events);
                        },
                        InputOrder::Serial(events) => {
                            shuffled_inputs.push(events);
                        }
                    }
                }

                test_scripts![shuffled_inputs, #(#scripts),*].await;
                tracing::info!("Suite used seed {seed}");
            }
        }
    };

    expanded.into()
}
