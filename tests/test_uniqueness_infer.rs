#[macro_use]
extern crate maplit;
#[macro_use]
extern crate pretty_assertions;
#[macro_use]
extern crate indoc;

extern crate bumpalo;
extern crate roc;

mod helpers;

#[cfg(test)]
mod test_infer_uniq {
    use crate::helpers::{assert_correct_variable_usage, can_expr, uniq_expr};
    use roc::can::ident::Lowercase;
    use roc::collections::{ImMap, ImSet};
    use roc::infer::infer_expr;
    use roc::pretty_print_types::{content_to_string, name_all_type_vars};
    use roc::subs::Subs;
    use roc::uniqueness::sharing::FieldAccess;
    use roc::uniqueness::sharing::ReferenceCount::{self, *};
    use roc::uniqueness::sharing::VarUsage;

    // HELPERS

    fn infer_eq_help(src: &str) -> (Vec<roc::types::Problem>, Subs, String) {
        let (_output, _problems, subs, variable, constraint) = uniq_expr(src);

        assert_correct_variable_usage(&constraint);

        let mut unify_problems = Vec::new();
        let (content, solved) = infer_expr(subs, &mut unify_problems, &constraint, variable);
        let mut subs = solved.into_inner();

        name_all_type_vars(variable, &mut subs);

        dbg!(&content, &subs);

        let actual_str = content_to_string(content, &mut subs);

        (unify_problems, subs, actual_str)
    }
    fn infer_eq_ignore_problems(src: &str, expected: &str) {
        let (_, _, actual) = infer_eq_help(src);

        assert_eq!(actual, expected.to_string());
    }

    fn infer_eq(src: &str, expected: &str) {
        let (problems, subs, actual) = infer_eq_help(src);

        if !problems.is_empty() {
            dbg!(&problems);
            dbg!(&subs);
            // fail with an assert, but print the problems normally so rust doesn't try to diff
            // an empty vec with the problems.
            println!("expected:\n{:?}\ninfered:\n{:?}", expected, actual);
            assert_eq!(0, 1);
        }
        assert_eq!(actual, expected.to_string());
    }

    #[test]
    fn empty_record() {
        infer_eq("{}", "Attr.Attr * {}");
    }

    #[test]
    fn int_literal() {
        infer_eq("5", "Attr.Attr * Int");
    }

    #[test]
    fn float_literal() {
        infer_eq("0.5", "Attr.Attr * Float");
    }

    #[test]
    fn string_literal() {
        infer_eq(
            indoc!(
                r#"
                "type inference!"
            "#
            ),
            "Attr.Attr * Str",
        );
    }

    #[test]
    fn empty_string() {
        infer_eq(
            indoc!(
                r#"
                ""
            "#
            ),
            "Attr.Attr * Str",
        );
    }

    // #[test]
    // fn block_string_literal() {
    //     infer_eq(
    //         indoc!(
    //             r#"
    //             """type
    //             inference!"""
    //         "#
    //         ),
    //         "Str",
    //     );
    // }

    // LIST

    #[test]
    fn empty_list_literal() {
        infer_eq(
            indoc!(
                r#"
                []
            "#
            ),
            "Attr.Attr * (List *)",
        );
    }

    #[test]
    fn list_of_lists() {
        infer_eq(
            indoc!(
                r#"
                [[]]
            "#
            ),
            "Attr.Attr * (List (Attr.Attr * (List *)))",
        );
    }

    #[test]
    fn triple_nested_list() {
        infer_eq(
            indoc!(
                r#"
                [[[]]]
            "#
            ),
            "Attr.Attr * (List (Attr.Attr * (List (Attr.Attr * (List *)))))",
        );
    }

    #[test]
    fn nested_empty_list() {
        infer_eq(
            indoc!(
                r#"
                [ [], [ [] ] ]
            "#
            ),
            "Attr.Attr * (List (Attr.Attr * (List (Attr.Attr * (List *)))))",
        );
    }

    // #[test]
    // fn concat_different_types() {
    //     infer_eq(
    //         indoc!(
    //             r#"
    //             empty = []
    //             one = List.concat [ 1 ] empty
    //             str = List.concat [ "blah" ] empty

    //             empty
    //         "#
    //         ),
    //         "List *",
    //     );
    // }

    #[test]
    fn list_of_one_int() {
        infer_eq(
            indoc!(
                r#"
                [42]
            "#
            ),
            "Attr.Attr * (List (Attr.Attr * Int))",
        );
    }

    #[test]
    fn triple_nested_int_list() {
        infer_eq(
            indoc!(
                r#"
                [[[ 5 ]]]
            "#
            ),
            "Attr.Attr * (List (Attr.Attr * (List (Attr.Attr * (List (Attr.Attr * Int))))))",
        );
    }

    #[test]
    fn list_of_ints() {
        infer_eq(
            indoc!(
                r#"
                [ 1, 2, 3 ]
            "#
            ),
            "Attr.Attr * (List (Attr.Attr * Int))",
        );
    }

    #[test]
    fn nested_list_of_ints() {
        infer_eq(
            indoc!(
                r#"
                [ [ 1 ], [ 2, 3 ] ]
            "#
            ),
            "Attr.Attr * (List (Attr.Attr * (List (Attr.Attr * Int))))",
        );
    }

    #[test]
    fn list_of_one_string() {
        infer_eq(
            indoc!(
                r#"
                [ "cowabunga" ]
            "#
            ),
            "Attr.Attr * (List (Attr.Attr * Str))",
        );
    }

    #[test]
    fn triple_nested_string_list() {
        infer_eq(
            indoc!(
                r#"
                [[[ "foo" ]]]
            "#
            ),
            "Attr.Attr * (List (Attr.Attr * (List (Attr.Attr * (List (Attr.Attr * Str))))))",
        );
    }

    #[test]
    fn list_of_strings() {
        infer_eq(
            indoc!(
                r#"
                [ "foo", "bar" ]
            "#
            ),
            "Attr.Attr * (List (Attr.Attr * Str))",
        );
    }

    // // INTERPOLATED STRING

    // #[test]
    // fn infer_interpolated_string() {
    //     infer_eq(
    //         indoc!(
    //             r#"
    //             whatItIs = "great"

    //             "type inference is \(whatItIs)!"
    //         "#
    //         ),
    //         "Str",
    //     );
    // }

    // LIST MISMATCH

    #[test]
    fn mismatch_heterogeneous_list() {
        infer_eq_ignore_problems(
            indoc!(
                r#"
                [ "foo", 5 ]
            "#
            ),
            "Attr.Attr * (List <type mismatch>)",
        );
    }

    #[test]
    fn mismatch_heterogeneous_nested_list() {
        infer_eq_ignore_problems(
            indoc!(
                r#"
                [ [ "foo", 5 ] ]
            "#
            ),
            "Attr.Attr * (List (Attr.Attr * (List <type mismatch>)))",
        );
    }

    #[test]
    fn mismatch_heterogeneous_nested_empty_list() {
        infer_eq_ignore_problems(
            indoc!(
                r#"
                [ [ 1 ], [ [] ] ]
            "#
            ),
            "Attr.Attr * (List <type mismatch>)",
        );
    }

    // CLOSURE

    #[test]
    fn always_return_empty_record() {
        infer_eq(
            indoc!(
                r#"
                \_ -> {}
            "#
            ),
            "Attr.Attr * (* -> Attr.Attr * {})",
        );
    }

    #[test]
    fn two_arg_return_int() {
        infer_eq(
            indoc!(
                r#"
                \_, _ -> 42
            "#
            ),
            "Attr.Attr * (*, * -> Attr.Attr * Int)",
        );
    }

    #[test]
    fn three_arg_return_string() {
        infer_eq(
            indoc!(
                r#"
                \_, _, _ -> "test!"
            "#
            ),
            "Attr.Attr * (*, *, * -> Attr.Attr * Str)",
        );
    }

    // DEF

    #[test]
    fn def_empty_record() {
        infer_eq(
            indoc!(
                r#"
                foo = {}

                foo
            "#
            ),
            "Attr.Attr * {}",
        );
    }

    #[test]
    fn def_string() {
        infer_eq(
            indoc!(
                r#"
                str = "thing"

                str
            "#
            ),
            "Attr.Attr * Str",
        );
    }

    #[test]
    fn def_1_arg_closure() {
        infer_eq(
            indoc!(
                r#"
                fn = \_ -> {}

                fn
            "#
            ),
            "Attr.Attr * (* -> Attr.Attr * {})",
        );
    }

    #[test]
    fn def_2_arg_closure() {
        infer_eq(
            indoc!(
                r#"
                func = \_, _ -> 42

                func
            "#
            ),
            "Attr.Attr * (*, * -> Attr.Attr * Int)",
        );
    }

    #[test]
    fn def_3_arg_closure() {
        infer_eq(
            indoc!(
                r#"
                f = \_, _, _ -> "test!"

                f
            "#
            ),
            "Attr.Attr * (*, *, * -> Attr.Attr * Str)",
        );
    }

    #[test]
    fn def_multiple_functions() {
        infer_eq(
            indoc!(
                r#"
                a = \_, _, _ -> "test!"

                b = a

                b
            "#
            ),
            "Attr.Attr * (*, *, * -> Attr.Attr * Str)",
        );
    }

    #[test]
    fn def_multiple_strings() {
        infer_eq(
            indoc!(
                r#"
                a = "test!"

                b = a

                b
            "#
            ),
            "Attr.Attr * Str",
        );
    }

    #[test]
    fn def_multiple_ints() {
        infer_eq(
            indoc!(
                r#"
                c = b

                b = a

                a = 42

                c
            "#
            ),
            "Attr.Attr * Int",
        );
    }

    // #[test]
    // fn def_returning_closure() {
    //     infer_eq(
    //         indoc!(
    //             r#"
    //             f = \z -> z
    //             g = \z -> z
    //
    //             (\x ->
    //                 a = f x
    //                 b = g x
    //                 x
    //             )
    //         "#
    //         ),
    //         // x is used 3 times, so must be shared
    //         "Attr.Attr * (Attr.Attr Attr.Shared a -> Attr.Attr Attr.Shared a)",
    //     );
    // }

    // CALLING FUNCTIONS

    #[test]
    fn call_returns_int() {
        infer_eq(
            indoc!(
                r#"
                alwaysFive = \_ -> 5

                alwaysFive "stuff"
                "#
            ),
            "Attr.Attr * Int",
        );
    }

    #[test]
    fn identity_returns_given_type() {
        infer_eq(
            indoc!(
                r#"
                identity = \a -> a

                identity "hi"
                "#
            ),
            "Attr.Attr * Str",
        );
    }

    #[test]
    fn identity_infers_principal_type() {
        infer_eq(
            indoc!(
                r#"
                    identity = \a -> a
                    x = identity 5

                    identity
                    "#
            ),
            "Attr.Attr Attr.Shared (a -> a)",
        );
    }

    #[test]
    fn identity_works_on_incompatible_types() {
        infer_eq(
            indoc!(
                r#"
                identity = \a -> a

                x = identity 5
                y = identity "hi"

                x
                "#
            ),
            // TODO investigate why is this not shared?
            // maybe because y is not used it is dropped?
            "Attr.Attr * Int",
        );
    }

    #[test]
    fn call_returns_list() {
        infer_eq(
            indoc!(
                r#"
                enlist = \val -> [ val ]

                enlist 5
                "#
            ),
            "Attr.Attr * (List (Attr.Attr * Int))",
        );
    }

    #[test]
    fn indirect_always() {
        infer_eq(
            indoc!(
                r#"
                    always = \val -> (\_ -> val)
                    alwaysFoo = always "foo"

                    alwaysFoo 42
                "#
            ),
            "Attr.Attr * Str",
        );
    }

    #[test]
    fn pizza_desugar() {
        infer_eq(
            indoc!(
                r#"
                1 |> (\a -> a)
                "#
            ),
            "Attr.Attr * Int",
        );
    }

    #[test]
    fn pizza_desugared() {
        infer_eq(
            indoc!(
                r#"
                (\a -> a) 1
                "#
            ),
            "Attr.Attr * Int",
        );
    }

    #[test]
    fn pizza_desugar_two_arguments() {
        infer_eq(
            indoc!(
                r#"
                always = \a, b -> a

                1 |> always "foo"
                "#
            ),
            "Attr.Attr * Int",
        );
    }

    #[test]
    fn anonymous_identity() {
        infer_eq(
            indoc!(
                r#"
                    (\a -> a) 3.14
                "#
            ),
            "Attr.Attr * Float",
        );
    }

    // TODO when symbols are unique, this should work again
    //    #[test]
    //    fn identity_of_identity() {
    //        infer_eq(
    //            indoc!(
    //                r#"
    //                    (\val -> val) (\val -> val)
    //                "#
    //            ),
    //            "Attr.Attr * (a -> a)",
    //        );
    //    }

    #[test]
    fn recursive_identity() {
        infer_eq(
            indoc!(
                r#"
                    identity = \val -> val

                    identity identity
                "#
            ),
            "Attr.Attr Attr.Shared (a -> a)",
        );
    }

    #[test]
    fn identity_function() {
        infer_eq(
            indoc!(
                r#"
                    \val -> val
                "#
            ),
            "Attr.Attr * (a -> a)",
        );
    }

    #[test]
    fn use_apply() {
        infer_eq(
            indoc!(
                r#"
                    apply = \f, x -> f x
                    identity = \a -> a

                    apply identity 5
                "#
            ),
            "Attr.Attr * Int",
        );
    }

    #[test]
    fn apply_function() {
        infer_eq(
            indoc!(
                r#"
                    \f, x -> f x
                "#
            ),
            "Attr.Attr * (Attr.Attr * (a -> b), a -> b)",
        );
    }

    // #[test]
    // TODO FIXME this should pass, but instead fails to canonicalize
    // fn use_flip() {
    //     infer_eq(
    //         indoc!(
    //             r#"
    //                 flip = \f -> (\a b -> f b a)
    //                 neverendingInt = \f int -> f int
    //                 x = neverendingInt (\a -> a) 5

    //                 flip neverendingInt
    //             "#
    //         ),
    //         "(Int, (a -> a)) -> Int",
    //     );
    // }

    #[test]
    fn flip_function() {
        infer_eq(
            indoc!(
                r#"
                    \f -> (\a, b -> f b a),
                "#
            ),
            "Attr.Attr * (Attr.Attr * (a, b -> c) -> Attr.Attr * (b, a -> c))",
        );
    }

    #[test]
    fn always_function() {
        infer_eq(
            indoc!(
                r#"
                    \val -> \_ -> val
                "#
            ),
            "Attr.Attr * (a -> Attr.Attr * (* -> a))",
        );
    }

    #[test]
    fn pass_a_function() {
        infer_eq(
            indoc!(
                r#"
                    \f -> f {}
                "#
            ),
            "Attr.Attr * (Attr.Attr * (Attr.Attr * {} -> a) -> a)",
        );
    }

    // OPERATORS

    // #[test]
    // fn div_operator() {
    //     infer_eq(
    //         indoc!(
    //             r#"
    //             \l r -> l / r
    //         "#
    //         ),
    //         "Float, Float -> Float",
    //     );
    // }

    //     #[test]
    //     fn basic_float_division() {
    //         infer_eq(
    //             indoc!(
    //                 r#"
    //                 1 / 2
    //             "#
    //             ),
    //             "Float",
    //         );
    //     }

    //     #[test]
    //     fn basic_int_division() {
    //         infer_eq(
    //             indoc!(
    //                 r#"
    //                 1 // 2
    //             "#
    //             ),
    //             "Int",
    //         );
    //     }

    //     #[test]
    //     fn basic_addition() {
    //         infer_eq(
    //             indoc!(
    //                 r#"
    //                 1 + 2
    //             "#
    //             ),
    //             "Int",
    //         );
    //     }

    // #[test]
    // fn basic_circular_type() {
    //     infer_eq(
    //         indoc!(
    //             r#"
    //             \x -> x x
    //         "#
    //         ),
    //         "<Type Mismatch: Circular Type>",
    //     );
    // }

    // #[test]
    // fn y_combinator_has_circular_type() {
    //     assert_eq!(
    //         infer(indoc!(r#"
    //             \f -> (\x -> f x x) (\x -> f x x)
    //         "#)),
    //         Erroneous(Problem::CircularType)
    //     );
    // }

    // #[test]
    // fn no_higher_ranked_types() {
    //     // This should error because it can't type of alwaysFive
    //     infer_eq(
    //         indoc!(
    //             r#"
    //             \always -> [ always [], always "" ]
    //        "#
    //         ),
    //         "<type mismatch>",
    //     );
    // }

    #[test]
    fn always_with_list() {
        infer_eq(
            indoc!(
                r#"
               alwaysFive = \_ -> 5

               [ alwaysFive "foo", alwaysFive [] ]
           "#
            ),
            "Attr.Attr * (List (Attr.Attr * Int))",
        );
    }

    #[test]
    fn if_with_int_literals() {
        infer_eq(
            indoc!(
                r#"
                if True then
                    42
                else
                    24
            "#
            ),
            "Attr.Attr * Int",
        );
    }

    #[test]
    fn when_with_int_literals() {
        infer_eq(
            indoc!(
                r#"
                when 1 is
                 1 -> 2
                 3 -> 4
            "#
            ),
            "Attr.Attr * Int",
        );
    }

    #[test]
    fn record() {
        infer_eq("{ foo: 42 }", "Attr.Attr * { foo : (Attr.Attr * Int) }");
    }

    #[test]
    fn record_access() {
        infer_eq("{ foo: 42 }.foo", "Attr.Attr * Int");
    }

    #[test]
    fn empty_record_pattern() {
        infer_eq(
            indoc!(
                r#"
                # technically, an empty record can be destructured
                {} = {}
                bar = \{} -> 42

                when foo is
                    { x: {} } -> x
            "#
            ),
            "Attr.Attr * {}*",
        );
    }

    #[test]
    fn record_update() {
        infer_eq(
            indoc!(
                r#"
                user = { year: "foo", name: "Sam" }

                { user & year: "foo" }
                "#
            ),
            "Attr.Attr * { name : (Attr.Attr * Str), year : (Attr.Attr * Str) }",
        );
    }

    #[test]
    fn bare_tag() {
        infer_eq(
            indoc!(
                r#"Foo
                "#
            ),
            "Attr.Attr * [ Foo ]*",
        );
    }

    #[test]
    fn single_tag_pattern() {
        infer_eq(
            indoc!(
                r#"\Foo -> 42
                "#
            ),
            "Attr.Attr * (Attr.Attr * [ Foo ]* -> Attr.Attr * Int)",
        );
    }

    #[test]
    fn single_private_tag_pattern() {
        infer_eq(
            indoc!(
                r#"\@Foo -> 42
                "#
            ),
            "Attr.Attr * (Attr.Attr * [ Test.@Foo ]* -> Attr.Attr * Int)",
        );
    }

    #[test]
    fn two_tag_pattern() {
        infer_eq(
            indoc!(
                r#"\x ->
                    when x is
                        True -> 1
                        False -> 0
                "#
            ),
            "Attr.Attr * (Attr.Attr * [ False, True ]* -> Attr.Attr * Int)",
        );
    }

    #[test]
    fn tag_application() {
        infer_eq(
            indoc!(
                r#"Foo "happy" 2020
                "#
            ),
            "Attr.Attr * [ Foo (Attr.Attr * Str) (Attr.Attr * Int) ]*",
        );
    }

    #[test]
    fn private_tag_application() {
        infer_eq(
            indoc!(
                r#"@Foo "happy" 2020
                "#
            ),
            "Attr.Attr * [ Test.@Foo (Attr.Attr * Str) (Attr.Attr * Int) ]*",
        );
    }

    #[test]
    fn record_field_accessor_function() {
        infer_eq(
            indoc!(
                r#"
                .left
                "#
            ),
            "Attr.Attr * (Attr.Attr (a | *) { left : (Attr.Attr a b) }* -> Attr.Attr a b)",
        );
    }

    #[test]
    fn record_field_access_syntax() {
        infer_eq(
            indoc!(
                r#"
                \rec -> rec.left
                "#
            ),
            "Attr.Attr * (Attr.Attr (a | *) { left : (Attr.Attr a b) }* -> Attr.Attr a b)",
        );
    }

    #[test]
    fn record_field_pattern_match_two() {
        infer_eq(
            indoc!(
                r#"
                \{ left, right } -> { left, right }
                "#
            ),
            "Attr.Attr * (Attr.Attr ((a | b) | *) { left : (Attr.Attr a c), right : (Attr.Attr b d) }* -> Attr.Attr * { left : (Attr.Attr a c), right : (Attr.Attr b d) })",
        );
    }

    #[test]
    fn record_field_pattern_match_with_guard() {
        infer_eq(
            indoc!(
                r#"
                    when foo is
                        { x: 4 } -> x
                "#
            ),
            "Attr.Attr * Int",
        );
    }

    #[test]
    fn tag_union_pattern_match() {
        infer_eq(
            indoc!(
                r#"
                \Foo x -> Foo x
                "#
            ),
            // NOTE: Foo loses the relation to the uniqueness attribute `a`
            // That is fine. Whenever we try to extract from it, the relation will be enforced
            "Attr.Attr * (Attr.Attr (a | *) [ Foo (Attr.Attr a b) ]* -> Attr.Attr * [ Foo (Attr.Attr a b) ]*)",
        );
    }

    #[test]
    fn tag_union_pattern_match_ignored_field() {
        infer_eq(
            indoc!(
                r#"
                \Foo x _ -> Foo x "y"
                "#
            ),
            // TODO: is it safe to ignore uniqueness constraints from patterns that bind no identifiers?
            // i.e. the `b` could be ignored in this example, is that true in general?
            // seems like it because we don't really extract anything.
            "Attr.Attr * (Attr.Attr ((a | b) | *) [ Foo (Attr.Attr a c) (Attr.Attr b *) ]* -> Attr.Attr * [ Foo (Attr.Attr a c) (Attr.Attr * Str) ]*)"
        );
    }

    #[test]
    fn global_tag_with_field() {
        infer_eq(
            indoc!(
                r#"
                    when Foo 4 is
                        Foo x -> x
                "#
            ),
            "Attr.Attr * Int",
        );
    }

    #[test]
    fn private_tag_with_field() {
        infer_eq(
            indoc!(
                r#"
                    when @Foo 4 is
                        @Foo x -> x
                "#
            ),
            "Attr.Attr * Int",
        );
    }

    #[test]
    fn type_annotation() {
        infer_eq(
            indoc!(
                r#"
                x : Num.Num Int.Integer
                x = 4

                x
                "#
            ),
            "Attr.Attr * Int",
        );
    }

    #[test]
    fn record_field_pattern_match() {
        infer_eq(
            indoc!(
                r#"
                \{ left } -> left
                "#
            ),
            "Attr.Attr * (Attr.Attr (a | *) { left : (Attr.Attr a b) }* -> Attr.Attr a b)",
        );
    }

    #[test]
    fn sharing_analysis_record_one_field_pattern() {
        infer_eq(
            indoc!(
                r#"
                \{ x } -> x
                "#
            ),
            "Attr.Attr * (Attr.Attr (a | *) { x : (Attr.Attr a b) }* -> Attr.Attr a b)",
        );
    }

    #[test]
    fn num_identity_def() {
        infer_eq(
            indoc!(
                r#"
                   numIdentity : Num.Num a -> Num.Num a
                   numIdentity = \x -> x

                   numIdentity
                   "#
            ),
            "Attr.Attr * (Attr.Attr a (Num b) -> Attr.Attr a (Num b))",
        );
    }

    #[test]
    fn record_field_access_binding() {
        infer_eq(
            indoc!(
                r#"
                \r ->
                    x = r.x

                    x
                "#
            ),
            "Attr.Attr * (Attr.Attr (a | *) { x : (Attr.Attr a b) }* -> Attr.Attr a b)",
        );
    }

    #[test]
    fn sharing_analysis_record_one_field_access() {
        infer_eq(
            indoc!(
                r#"
                \r ->
                    x = r.x

                    x
                "#
            ),
            "Attr.Attr * (Attr.Attr (a | *) { x : (Attr.Attr a b) }* -> Attr.Attr a b)",
        );
    }

    #[test]
    fn num_identity_applied() {
        infer_eq(
            indoc!(
                r#"
                   numIdentity : Num.Num b -> Num.Num b
                   numIdentity = \foo -> foo

                   p = numIdentity 42
                   q = numIdentity 3.14

                   { numIdentity, p, q }
                   "#
            ), "Attr.Attr * { numIdentity : (Attr.Attr Attr.Shared (Attr.Attr a (Num b) -> Attr.Attr a (Num b))), p : (Attr.Attr * Int), q : (Attr.Attr * Float) }"
        );
    }

    #[test]
    fn sharing_analysis_record_twice_access() {
        infer_eq(
                    indoc!(
                        r#"
                        \r -> 
                            v = r.x
                            w = r.x

                            r

                        "#
                    ),
                "Attr.Attr * (Attr.Attr a { x : (Attr.Attr Attr.Shared b) }c -> Attr.Attr a { x : (Attr.Attr Attr.Shared b) }c)" ,
                );
    }

    #[test]
    fn sharing_analysis_record_access_two_fields() {
        infer_eq(
                    indoc!(
                        r#"
                        \r -> 
                            v = r.x
                            w = r.y

                            r

                        "#
                    ),
                "Attr.Attr * (Attr.Attr a { x : (Attr.Attr Attr.Shared b), y : (Attr.Attr Attr.Shared c) }d -> Attr.Attr a { x : (Attr.Attr Attr.Shared b), y : (Attr.Attr Attr.Shared c) }d)",
                );
    }

    #[test]
    fn sharing_analysis_record_alias() {
        infer_eq(
                    indoc!(
                        r#"
                        \r -> 
                            v = r.x
                            w = r.y

                            p = r

                            p
                        "#
                    ),
                "Attr.Attr * (Attr.Attr Attr.Shared { x : (Attr.Attr Attr.Shared a), y : (Attr.Attr Attr.Shared b) }c -> Attr.Attr Attr.Shared { x : (Attr.Attr Attr.Shared a), y : (Attr.Attr Attr.Shared b) }c)"
                );
    }

    #[test]
    fn sharing_analysis_record_access_field_twice() {
        infer_eq(
            indoc!(
                r#"
                \r ->
                    n = r.x
                    m = r.x

                    r
                        "#
            ),
            "Attr.Attr * (Attr.Attr a { x : (Attr.Attr Attr.Shared b) }c -> Attr.Attr a { x : (Attr.Attr Attr.Shared b) }c)",
        );
    }

    #[test]
    fn sharing_analysis_record_update_duplicate_field() {
        infer_eq(
            indoc!(
                r#"
                \r -> { r & x: r.x, y: r.x }
                "#
            ),
         "Attr.Attr * (Attr.Attr a { x : (Attr.Attr Attr.Shared b), y : (Attr.Attr Attr.Shared b) }c -> Attr.Attr a { x : (Attr.Attr Attr.Shared b), y : (Attr.Attr Attr.Shared b) }c)"
        );
    }

    #[test]
    fn record_access_nested_field() {
        infer_eq(
            indoc!(
                r#"
                \r -> 
                    v = r.foo.bar
                    w = r.foo.baz

                    r
                "#
            ),
            "Attr.Attr * (Attr.Attr (a | b) { foo : (Attr.Attr a { bar : (Attr.Attr Attr.Shared d), baz : (Attr.Attr Attr.Shared c) }e) }f -> Attr.Attr (b | a) { foo : (Attr.Attr a { bar : (Attr.Attr Attr.Shared d), baz : (Attr.Attr Attr.Shared c) }e) }f)"
        );
    }

    #[test]
    fn record_access_nested_field_is_safe() {
        infer_eq(
            indoc!(
                r#"
                \r -> 
                    v = r.foo.bar

                    x = v
                    y = v

                    r
                "#
            ),
            "Attr.Attr * (Attr.Attr (a | b) { foo : (Attr.Attr a { bar : (Attr.Attr Attr.Shared c) }d) }e -> Attr.Attr (b | a) { foo : (Attr.Attr a { bar : (Attr.Attr Attr.Shared c) }d) }e)"
        );
    }

    #[test]
    fn record_update_is_safe() {
        infer_eq(
            indoc!(
                r#"
                \r ->

                    s = { r & y: r.x }

                    p = s.x
                    q = s.y

                    s
                "#
            ),
            "Attr.Attr * (Attr.Attr a { x : (Attr.Attr Attr.Shared b), y : (Attr.Attr Attr.Shared b) }c -> Attr.Attr a { x : (Attr.Attr Attr.Shared b), y : (Attr.Attr Attr.Shared b) }c)",
        );
    }

    #[test]
    fn when_with_annotation() {
        infer_eq(
            indoc!(
                r#"
                    x : Num.Num Int.Integer
                    x =
                        when 2 is
                            3 -> 4
                            _ -> 5

                    x
                   "#
            ),
            "Attr.Attr * Int",
        );
    }

    // TODO add more realistic recursive example when able
    #[test]
    fn factorial_is_shared() {
        infer_eq(
            indoc!(
                r#"
                    factorial = \n ->
                        when n is
                            0 -> 1
                            1 -> 1
                            m -> factorial m

                    factorial
                   "#
            ),
            "Attr.Attr Attr.Shared (Attr.Attr * Int -> Attr.Attr * Int)",
        );
    }

    // TODO add more realistic recursive example when able
    #[test]
    fn factorial_without_recursive_case_can_be_unique() {
        infer_eq(
            indoc!(
                r#"
                    factorial = \n ->
                        when n is
                            0 -> 1
                            _ -> 1

                    factorial
                   "#
            ),
            "Attr.Attr * (Attr.Attr * Int -> Attr.Attr * Int)",
        );
    }

    fn field_access_seq(
        accesses: Vec<Vec<&str>>,
        expected: std::collections::HashMap<&str, ReferenceCount>,
    ) {
        let mut state = FieldAccess::default();

        for access in accesses {
            let temp: Vec<Lowercase> = access.into_iter().map(|v| v.into()).collect();
            state.sequential(temp);
        }

        let mut im_expected: std::collections::HashMap<String, ReferenceCount> =
            std::collections::HashMap::default();

        for (k, v) in expected {
            im_expected.insert(k.into(), v);
        }

        let actual: std::collections::HashMap<String, ReferenceCount> = state.into();

        assert_eq!(actual, im_expected);
    }

    fn field_access_par(
        accesses: Vec<Vec<&str>>,
        expected: std::collections::HashMap<&str, ReferenceCount>,
    ) {
        let mut state = FieldAccess::default();

        for access in accesses {
            let temp: Vec<Lowercase> = access.into_iter().map(|v| v.into()).collect();
            state.parallel(temp);
        }

        let mut im_expected: std::collections::HashMap<String, ReferenceCount> =
            std::collections::HashMap::default();

        for (k, v) in expected {
            im_expected.insert(k.into(), v);
        }

        let actual: std::collections::HashMap<String, ReferenceCount> = state.into();

        assert_eq!(actual, im_expected);
    }

    #[test]
    fn usage_access_two_fields() {
        field_access_seq(
            vec![vec!["foo"], vec!["bar"]],
            hashmap![
                "foo" => Unique,
                "bar" => Unique,
            ],
        );

        field_access_par(
            vec![vec!["foo"], vec!["bar"]],
            hashmap![
                "foo" => Unique,
                "bar" => Unique,
            ],
        );
    }

    #[test]
    fn usage_access_repeated_field_seq() {
        field_access_seq(
            vec![vec!["foo"], vec!["foo"]],
            hashmap![
                "foo" => Shared,
            ],
        );
    }

    #[test]
    fn usage_access_repeated_field_par() {
        field_access_par(
            vec![vec!["foo"], vec!["foo"]],
            hashmap![
                "foo" => Unique,
            ],
        );
    }

    #[test]
    fn usage_access_nested_field_seq() {
        field_access_seq(
            vec![vec!["foo", "bar"], vec!["foo"]],
            hashmap![
                "foo" => Unique,
                "foo.bar" => Shared,
            ],
        );
        field_access_seq(
            vec![vec!["foo"], vec!["foo", "bar"]],
            hashmap![
                "foo" => Unique,
                "foo.bar" => Shared,
            ],
        );
    }
    #[test]
    fn usage_access_nested_field_par() {
        field_access_par(
            vec![vec!["foo", "bar"], vec!["foo"]],
            hashmap![
                "foo" => Unique,
                "foo.bar" => Unique,
            ],
        );
        field_access_par(
            vec![vec!["foo"], vec!["foo", "bar"]],
            hashmap![
                "foo" => Unique,
                "foo.bar" => Unique,
            ],
        );
    }

    #[test]
    fn usage_access_deeply_nested_field_seq() {
        field_access_seq(
            vec![vec!["foo", "bar", "baz"], vec!["foo", "bar"]],
            hashmap![
                "foo" => Seen,
                "foo.bar" => Unique,
                "foo.bar.baz" => Shared,
            ],
        );
        field_access_seq(
            vec![vec!["foo", "bar"], vec!["foo", "bar", "baz"]],
            hashmap![
                "foo" => Seen,
                "foo.bar" => Unique,
                "foo.bar.baz" => Shared,
            ],
        );
    }
    fn usage_eq(src: &str, expected: VarUsage) {
        let (expr, _, _problems, _subs, _variable, _constraint) = can_expr(src);

        use roc::uniqueness::sharing::annotate_usage;
        let mut usage = VarUsage::default();
        annotate_usage(&expr, &mut usage);

        dbg!(&usage);

        assert_eq!(usage, expected)
    }

    #[test]
    fn usage_factorial() {
        usage_eq(
            indoc!(
                r#"
                    factorial = \n ->
                        when n is
                            0 -> 1
                            1 -> 1
                            m -> factorial m

                    factorial
                   "#
            ),
            {
                let mut usage = VarUsage::default();

                usage.register_with(&"Test.blah$m".into(), &Unique);
                usage.register_with(&"Test.blah$n".into(), &Unique);
                usage.register_with(&"Test.blah$factorial".into(), &Shared);

                usage
            },
        );
    }

    #[test]
    fn usage_record_access() {
        usage_eq(
            indoc!(
                r#"
            rec = { foo : 42, bar : "baz" } 
            rec.foo
                   "#
            ),
            {
                let mut usage = VarUsage::default();
                let fa = FieldAccess::from_chain(vec!["foo".into()]);

                usage.register_with(&"Test.blah$rec".into(), &ReferenceCount::Access(fa));

                usage
            },
        );
    }

    #[test]
    fn usage_record_update() {
        usage_eq(
            indoc!(
                r#"
            rec = { foo : 42, bar : "baz" } 
            { rec & foo: rec.foo } 
                   "#
            ),
            {
                let mut usage = VarUsage::default();
                let fa = FieldAccess::from_chain(vec!["foo".into()]);

                let overwritten = hashset!["foo".into()].into();
                usage.register_with(
                    &"Test.blah$rec".into(),
                    &ReferenceCount::Update(overwritten, fa),
                );

                usage
            },
        );
    }

    #[test]
    fn update_then_unique() {
        usage_eq(
            indoc!(
                r#"
            rec = { foo : 42, bar : "baz" } 
            v = { rec & foo: 53 }

            rec
                   "#
            ),
            {
                let mut usage = VarUsage::default();
                usage.register_with(&"Test.blah$rec".into(), &ReferenceCount::Shared);

                usage
            },
        );
    }

    #[test]
    fn access_then_unique() {
        usage_eq(
            indoc!(
                r#"
            rec = { foo : 42, bar : "baz" } 
            v = rec.foo

            rec
                   "#
            ),
            {
                let mut usage = VarUsage::default();
                let mut fields = ImMap::default();
                fields.insert(
                    "foo".into(),
                    (ReferenceCount::Shared, FieldAccess::default()),
                );
                let fa = FieldAccess { fields: fields };
                usage.register_with(
                    &"Test.blah$rec".into(),
                    &ReferenceCount::Update(ImSet::default(), fa),
                );

                usage
            },
        );
    }

    #[test]
    fn access_then_alias() {
        usage_eq(
            indoc!(
                r#"
                        \r -> 
                            v = r.x
                            w = r.y

                            p = r

                            p
                   "#
            ),
            {
                let mut usage = VarUsage::default();
                let mut fields = ImMap::default();
                fields.insert(
                    "foo".into(),
                    (ReferenceCount::Shared, FieldAccess::default()),
                );
                let fa = FieldAccess { fields: fields };
                usage.register_with(&"Test.blah$r".into(), &ReferenceCount::Shared);
                usage.register_with(&"Test.blah$p".into(), &ReferenceCount::Unique);

                usage
            },
        );
    }

    #[test]
    fn access_nested_then_unique() {
        usage_eq(
            indoc!(
                r#"
                \r -> 
                    v = r.foo.bar
                    w = r.foo.baz

                    r
                   "#
            ),
            {
                let mut usage = VarUsage::default();

                let mut nested_fields = ImMap::default();
                nested_fields.insert(
                    "bar".into(),
                    (ReferenceCount::Shared, FieldAccess::default()),
                );
                nested_fields.insert(
                    "baz".into(),
                    (ReferenceCount::Shared, FieldAccess::default()),
                );
                let nested_fa = FieldAccess {
                    fields: nested_fields,
                };

                let mut fields = ImMap::default();
                fields.insert("foo".into(), (ReferenceCount::Seen, nested_fa));

                let fa = FieldAccess { fields: fields };
                usage.register_with(
                    &"Test.blah$r".into(),
                    &ReferenceCount::Update(ImSet::default(), fa),
                );

                usage
            },
        );
    }

    #[test]
    fn usage_record_update_unique_not_overwritten() {
        usage_eq(
            indoc!(
                r#"
            r = { x : 42, y : 2020 }
            s = { r & y: r.x }

            p = s.x
            q = s.y

            42
                   "#
            ),
            {
                let mut usage = VarUsage::default();

                let mut fields = ImMap::default();
                fields.insert("x".into(), (ReferenceCount::Shared, FieldAccess::default()));
                let fa = FieldAccess { fields: fields };
                let overwritten = hashset!["y".into()].into();
                usage.register_with(
                    &"Test.blah$r".into(),
                    &ReferenceCount::Update(overwritten, fa),
                );

                let mut fields = ImMap::default();
                fields.insert("x".into(), (ReferenceCount::Unique, FieldAccess::default()));
                fields.insert("y".into(), (ReferenceCount::Unique, FieldAccess::default()));
                let fa = FieldAccess { fields: fields };
                usage.register_with(&"Test.blah$s".into(), &ReferenceCount::Access(fa));

                usage
            },
        );
    }

    #[test]
    fn usage_record_update_unique_overwritten() {
        usage_eq(
            indoc!(
                r#"
            r = { x : 42, y : 2020 } 
            s = { r & x: 0, y: r.x }

            p = s.x
            q = s.y

            42
                   "#
            ),
            {
                // pub fields: ImMap<String, (ReferenceCount, FieldAccess)>,
                let mut usage = VarUsage::default();

                let fa = FieldAccess::from_chain(vec!["x".into()]);
                let overwritten = hashset!["x".into(), "y".into()].into();
                usage.register_with(
                    &"Test.blah$r".into(),
                    &ReferenceCount::Update(overwritten, fa),
                );

                let mut fields = ImMap::default();
                fields.insert("x".into(), (ReferenceCount::Unique, FieldAccess::default()));
                fields.insert("y".into(), (ReferenceCount::Unique, FieldAccess::default()));
                let fa = FieldAccess { fields: fields };
                usage.register_with(&"Test.blah$s".into(), &ReferenceCount::Access(fa));

                usage
            },
        );
    }

    // TODO when symbols are unique, ensure each `val` is counted only once
    //    #[test]
    //    fn usage_closures_with_same_bound_name() {
    //        usage_eq(
    //            indoc!(
    //                r#"
    //                    (\val -> val) (\val -> val)
    //                "#
    //            ),
    //            {
    //                let mut usage = VarUsage::default();
    //                let fa = FieldAccess::from_chain(vec!["foo".into()]);
    //
    //                usage.register_with(&"Test.blah$rec".into(), &ReferenceCount::Update(fa));
    //
    //                usage
    //            },
    //        );
    //    }
}
