-module(gleam@pair_test).
-compile(no_auto_import).

-export([first_test/0, second_test/0, swap_test/0]).

first_test() ->
    gleam@expect:equal(gleam@pair:first({1, 2}), 1),
    gleam@expect:equal(gleam@pair:first({<<"abc">>, []}), <<"abc">>).

second_test() ->
    gleam@expect:equal(gleam@pair:second({1, 2}), 2),
    gleam@expect:equal(gleam@pair:second({<<"abc">>, []}), []).

swap_test() ->
    gleam@expect:equal(gleam@pair:swap({1, <<"2">>}), {<<"2">>, 1}).
