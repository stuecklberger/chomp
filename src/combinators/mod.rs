//! Basic combinators.

#[macro_use]
mod macros;

pub mod bounded;

use std::iter::FromIterator;

use {ParseResult, Input};

use primitives::State;
use primitives::{IntoInner, InputBuffer, InputClone};

/// Applies the parser ``p`` exactly ``num`` times, propagating any error or incomplete state.
///
#[cfg_attr(feature = "verbose_error", doc = "
```
use chomp::{ParseResult, Error, Input, count, token, take_remainder};

let p1 = Input::new(b\"a  \");
let p2 = Input::new(b\"aa \");
let p3 = Input::new(b\"aaa\");

fn parse(i: Input<u8>) -> ParseResult<u8, Vec<u8>, Error<u8>> {
    count(i, 2, |i| token(i, b'a'))
}

assert_eq!(parse(p1).unwrap_err(), Error::Expected(b'a'));
assert_eq!(parse(p2).unwrap(), &[b'a', b'a']);

// TODO: Update once a proper way to extract data and remainder has been implemented
// a slightly odd way to obtain the remainder of the input stream, temporary:
let d: ParseResult<_, (_, Vec<_>), Error<_>> =
    parse(p3).bind(|i, d| take_remainder(i).bind(|i, r| i.ret((r, d))));
let (buf, data) = d.unwrap();

assert_eq!(buf, b\"a\");
assert_eq!(data, &[b'a', b'a']);
```
")]
#[inline]
pub fn count<'a, I, T, E, F, U>(i: Input<'a, I>, num: usize, p: F) -> ParseResult<'a, I, T, E>
  where I: Copy,
        U: 'a,
        F: FnMut(Input<'a, I>) -> ParseResult<'a, I, U, E>,
        T: FromIterator<U> {
    bounded::many(i, num, p)
}

/// Tries the parser ``f``, on success it yields the parsed value, on failure ``default`` will be
/// yielded instead.
///
/// Incomplete state is propagated. Backtracks on error.
///
/// ```
/// use chomp::{Input, option, token};
///
/// let p1 = Input::new(b"abc");
/// let p2 = Input::new(b"bbc");
///
/// assert_eq!(option(p1, |i| token(i, b'a'), b'd').unwrap(), b'a');
/// assert_eq!(option(p2, |i| token(i, b'a'), b'd').unwrap(), b'd');
/// ```
#[inline]
pub fn option<'a, I, T, E, F>(i: Input<'a, I>, f: F, default: T) -> ParseResult<'a, I, T, E>
  where I: 'a + Copy,
        F: FnOnce(Input<'a, I>) -> ParseResult<'a, I, T, E> {
    match f(i.clone()).into_inner() {
        State::Data(b, d)    => b.ret(d),
        State::Error(_, _)   => i.ret(default),
        State::Incomplete(n) => i.incomplete(n),
    }
}

/// Tries to match the parser ``f``, if ``f`` fails it tries ``g``. Returns the success value of
/// the first match, otherwise the error of the last one if both fail.
///
/// Incomplete state is propagated from the first one to report incomplete.
///
#[cfg_attr(feature = "verbose_error", doc = "
```
use chomp::{Input, Error, or, token};

let p1 = Input::new(b\"abc\");
let p2 = Input::new(b\"bbc\");
let p3 = Input::new(b\"cbc\");

assert_eq!(or(p1, |i| token(i, b'a'), |i| token(i, b'b')).unwrap(), b'a');
assert_eq!(or(p2, |i| token(i, b'a'), |i| token(i, b'b')).unwrap(), b'b');
assert_eq!(or(p3, |i| token(i, b'a'), |i| token(i, b'b')).unwrap_err(), Error::Expected(b'b'));
```
")]
#[inline]
pub fn or<'a, I, T, E, F, G>(i: Input<'a, I>, f: F, g: G) -> ParseResult<'a, I, T, E>
  where F: FnOnce(Input<'a, I>) -> ParseResult<'a, I, T, E>,
        G: FnOnce(Input<'a, I>) -> ParseResult<'a, I, T, E> {
    match f(i.clone()).into_inner() {
        State::Data(b, d)    => b.ret(d),
        State::Error(_, _)   => g(i),
        State::Incomplete(n) => i.incomplete(n),
    }
}

/// Parses many instances of ``f`` until it does no longer match, returning all matches.
///
/// Note: If the last parser succeeds on the last input item then this parser is still considered
/// incomplete if the input flag END_OF_INPUT is not set as there might be more data to fill.
///
/// Note: Allocates data.
///
/// ```
/// use chomp::{ParseResult, Error, Input, token, many, take_while1};
///
/// let p = Input::new(b"a,bc,cd ");
///
/// let r: ParseResult<_, Vec<&[u8]>, Error<u8>> =
///     many(p, |i| take_while1(i, |c| c != b',' && c != b' ').bind(|i, c|
///         token(i, b',').bind(|i, _| i.ret(c))));
/// let v = r.unwrap();
///
/// assert_eq!(v.len(), 2);
/// assert_eq!(v[0], b"a");
/// assert_eq!(v[1], b"bc");
/// ```
#[inline]
pub fn many<'a, I, T, E, F, U>(i: Input<'a, I>, f: F) -> ParseResult<'a, I, T, E>
  where I: Copy,
        U: 'a,
        F: FnMut(Input<'a, I>) -> ParseResult<'a, I, U, E>,
        T: FromIterator<U> {
    bounded::many(i, .., f)
}

/// Parses at least one instance of ``f`` and continues until it does no longer match,
/// returning all matches.
///
/// Note: If the last parser succeeds on the last input item then this parser is still considered
/// incomplete as there might be more data to fill.
///
/// Note: Allocates data.
///
#[cfg_attr(feature = "verbose_error", doc = "
```
use chomp::{ParseResult, Error, Input, token, many1, take_while1};

let p1 = Input::new(b\"a \");
let p2 = Input::new(b\"a, \");

fn parse(i: Input<u8>) -> ParseResult<u8, Vec<&[u8]>, Error<u8>> {
    many1(i, |i| take_while1(i, |c| c != b',' && c != b' ').bind(|i, c|
        token(i, b',').bind(|i, _| i.ret(c))))
}

assert_eq!(parse(p1).unwrap_err(), Error::Expected(b','));
assert_eq!(parse(p2).unwrap(), &[b\"a\"]);
```
")]
#[inline]
pub fn many1<'a, I, T, E, F, U>(i: Input<'a, I>, f: F) -> ParseResult<'a, I, T, E>
  where I: Copy,
        U: 'a,
        F: FnMut(Input<'a, I>) -> ParseResult<'a, I, U, E>,
        T: FromIterator<U> {
    bounded::many(i, 1.., f)
}

/// Applies the parser `R` zero or more times, separated by the parser `F`. All matches from `R`
/// will be collected into the type `T` implementing `IntoIterator`.
///
/// If the separator or parser registers error or incomplete this parser stops and yields the
/// collected value.
///
/// Incomplete will be propagated from `R` if end of input has not been read.
///
/// ```
/// use chomp::{Input, sep_by, token};
/// use chomp::ascii::decimal;
///
/// let i = Input::new(b"91;03;20");
///
/// let r: Vec<u8> = sep_by(i, decimal, |i| token(i, b';')).unwrap();
///
/// assert_eq!(r, vec![91, 03, 20]);
/// ```
#[inline]
pub fn sep_by<'a, I, T, E, R, F, U, N, V>(i: Input<'a, I>, mut p: R, mut sep: F) -> ParseResult<'a, I, T, E>
  where I: Copy,
        U: 'a,
        V: 'a,
        N: 'a,
        T: FromIterator<U>,
        E: From<N>,
        R: FnMut(Input<'a, I>) -> ParseResult<'a, I, U, E>,
        F: FnMut(Input<'a, I>) -> ParseResult<'a, I, V, N> {
    // If we have parsed at least one item
    let mut item = false;
    // Add sep in front of p if we have read at least one item
    let parser   = |i| (if item {
            sep(i).map(|_| ())
        } else {
            i.ret(())
        })
        .then(&mut p)
        .inspect(|_| item = true);

    bounded::many(i, .., parser)
}


/// Applies the parser `R` one or more times, separated by the parser `F`. All matches from `R`
/// will be collected into the type `T` implementing `IntoIterator`.
///
/// If the separator or parser registers error or incomplete this parser stops and yields the
/// collected value if at least one item has been read.
///
/// Incomplete will be propagated from `R` if end of input has not been read.
///
/// ```
/// use chomp::{Input, sep_by1, token};
/// use chomp::ascii::decimal;
///
/// let i = Input::new(b"91;03;20");
///
/// let r: Vec<u8> = sep_by1(i, decimal, |i| token(i, b';')).unwrap();
///
/// assert_eq!(r, vec![91, 03, 20]);
/// ```
#[inline]
pub fn sep_by1<'a, I, T, E, R, F, U, N, V>(i: Input<'a, I>, mut p: R, mut sep: F) -> ParseResult<'a, I, T, E>
  where I: Copy,
        U: 'a,
        V: 'a,
        N: 'a,
        T: FromIterator<U>,
        E: From<N>,
        R: FnMut(Input<'a, I>) -> ParseResult<'a, I, U, E>,
        F: FnMut(Input<'a, I>) -> ParseResult<'a, I, V, N> {
    // If we have parsed at least one item
    let mut item = false;
    // Add sep in front of p if we have read at least one item
    let parser   = |i| (if item {
            sep(i).map(|_| ())
        } else {
            i.ret(())
        })
        .then(&mut p)
        .inspect(|_| item = true);

    bounded::many(i, 1.., parser)
}

/// Applies the parser `R` multiple times until the parser `F` succeeds and returns a value
/// populated by the values yielded by `R`. Consumes the matched part of `F`.
///
/// This parser is considered incomplete if the parser `R` is considered incomplete.
///
/// Errors from `R` are propagated.
///
/// ```
/// use chomp::{Input, ParseResult, many_till, any, token};
///
/// let i = Input::new(b"abc;def");
///
/// let r: ParseResult<_, Vec<u8>, _> = many_till(i, any, |i| token(i, b';'));
///
/// assert_eq!(r.unwrap(), vec![b'a', b'b', b'c']);
/// ```
#[inline]
pub fn many_till<'a, I, T, E, R, F, U, N, V>(i: Input<'a, I>, p: R, end: F) -> ParseResult<'a, I, T, E>
  where I: Copy,
        U: 'a,
        V: 'a,
        N: 'a,
        T: FromIterator<U>,
        R: FnMut(Input<'a, I>) -> ParseResult<'a, I, U, E>,
        F: FnMut(Input<'a, I>) -> ParseResult<'a, I, V, N> {
    bounded::many_till(i, .., p, end)
}

/// Runs the given parser until it fails, discarding matched input.
///
/// Incomplete state will be propagated.
///
/// This is more efficient compared to using ``many`` and then just discarding the result as
/// ``many`` allocates a separate data structure to contain the data before proceeding.
///
/// ```
/// use chomp::{Input, skip_many, token};
///
/// let p = Input::new(b"aaaabc");
///
/// assert_eq!(skip_many(p, |i| token(i, b'a')).bind(|i, _| token(i, b'b')).unwrap(), b'b');
/// ```
#[inline]
pub fn skip_many<'a, I, T, E, F>(i: Input<'a, I>, f: F) -> ParseResult<'a, I, (), E>
  where T: 'a,
        F: FnMut(Input<'a, I>) -> ParseResult<'a, I, T, E> {
    bounded::skip_many(i, .., f)
}

/// Runs the given parser until it fails, discarding matched input, expects at least one match.
///
/// Incomplete state will be propagated. Will propagate the error if it occurs during the first
/// attempt.
///
/// This is more efficient compared to using ``many1`` and then just discarding the result as
/// ``many1`` allocates a separate data structure to contain the data before proceeding.
///
/// ```
/// use chomp::{Input, skip_many1, token};
///
/// let i1 = Input::new(b"aaaabc");
/// let i2 = Input::new(b"abc");
///
/// let p = |i| skip_many1(i, |i| token(i, b'a')).bind(|i, _| token(i, b'b'));
///
/// assert_eq!(p(i1).unwrap(), b'b');
/// assert_eq!(p(i2).unwrap(), b'b');
/// ```
///
/// ```should_panic
/// use chomp::{Input, skip_many1, token};
///
/// let i = Input::new(b"bc");
///
/// let p = |i| skip_many1(i, |i| token(i, b'a')).bind(|i, _| token(i, b'b'));
///
/// // Error:
/// assert_eq!(p(i).unwrap(), b'b');
/// ```
#[inline]
pub fn skip_many1<'a, I, T, E, F>(i: Input<'a, I>, f: F) -> ParseResult<'a, I, (), E>
  where T: 'a, F: FnMut(Input<'a, I>) -> ParseResult<'a, I, T, E> {
    bounded::skip_many(i, 1.., f)
}

/// Returns the result of the given parser as well as the slice which matched it.
///
/// ```
/// use chomp::{Input, matched_by};
/// use chomp::ascii::decimal;
///
/// let i = Input::new(b"123");
///
/// assert_eq!(matched_by(i, decimal).unwrap(), (&b"123"[..], 123u32));
/// ```
#[inline]
pub fn matched_by<'a, I, T, E, F>(i: Input<'a, I>, f: F) -> ParseResult<'a, I, (&'a [I], T), E>
  where T: 'a,
        F: FnOnce(Input<'a, I>) -> ParseResult<'a, I, T, E> {
    let buf = i.buffer();

    match f(i.clone()).into_inner() {
        State::Data(b, t) => {
            // b is remainder, find out how much the parser used
            let diff = buf.len() - b.buffer().len();
            let n    = &buf[..diff];

            b.ret((n, t))
        },
        State::Error(b, e)   => i.replace(b).err(e),
        State::Incomplete(n) => i.incomplete(n),
    }
}

/// Applies the parser `F` without consuming any input.
///
/// ```
/// use chomp::{Input, take};
/// use chomp::combinators::look_ahead;
///
/// let i = Input::new(b"testing");
///
/// let r = look_ahead(i, |i| take(i, 4)).bind(|i, t| take(i, 7).map(|u| (t, u)));
///
/// assert_eq!(r.unwrap(), (&b"test"[..], &b"testing"[..]));
/// ```
#[inline]
pub fn look_ahead<'a, I, T, E, F>(i: Input<'a, I>, f: F) -> ParseResult<'a, I, T, E>
  where F: FnOnce(Input<'a, I>) -> ParseResult<'a, I, T, E> {
    match f(i.clone()).into_inner() {
        State::Data(_, t)    => i.ret(t),
        State::Error(b, t)   => i.replace(b).err(t),
        State::Incomplete(n) => i.incomplete(n),
    }
}

#[cfg(test)]
mod test {
    use ParseResult;
    use primitives::State;
    use primitives::input::{new, DEFAULT, END_OF_INPUT};
    use primitives::IntoInner;
    use super::*;

    use parsers::{any, token, string};

    #[test]
    fn many_test() {
        let r: State<_, Vec<_>, _> = many(new(DEFAULT, b""), |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Incomplete(1));
        let r: State<_, Vec<_>, _> = many(new(DEFAULT, b"a"), |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Incomplete(1));
        let r: State<_, Vec<_>, _> = many(new(DEFAULT, b"aa"), |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Incomplete(1));

        let r: State<_, Vec<_>, _> = many(new(DEFAULT, b"bbb"), |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Data(new(DEFAULT, b"bbb"), vec![]));
        let r: State<_, Vec<_>, _> = many(new(DEFAULT, b"abb"), |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Data(new(DEFAULT, b"bb"), vec![b'a']));
        let r: State<_, Vec<_>, _> = many(new(DEFAULT, b"aab"), |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Data(new(DEFAULT, b"b"), vec![b'a', b'a']));

        let r: State<_, Vec<_>, _> = many(new(END_OF_INPUT, b""), |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Data(new(END_OF_INPUT, b""), vec![]));
        let r: State<_, Vec<_>, _> = many(new(END_OF_INPUT, b"a"), |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Data(new(END_OF_INPUT, b""), vec![b'a']));
        let r: State<_, Vec<_>, _> = many(new(END_OF_INPUT, b"aa"), |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Data(new(END_OF_INPUT, b""), vec![b'a', b'a']));

        let r: State<_, Vec<_>, _> = many(new(END_OF_INPUT, b"aab"), |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Data(new(END_OF_INPUT, b"b"), vec![b'a', b'a']));
    }

    #[test]
    fn many1_test() {
        let r: State<_, Vec<_>, _> = many1(new(DEFAULT, b""), |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Incomplete(1));
        let r: State<_, Vec<_>, _> = many1(new(DEFAULT, b"a"), |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Incomplete(1));
        let r: State<_, Vec<_>, _> = many1(new(DEFAULT, b"aa"), |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Incomplete(1));

        let r: State<_, Vec<_>, _> = many1(new(DEFAULT, b"bbb"), |i| token(i, b'a').map_err(|_| "token_error")).into_inner();
        assert_eq!(r, State::Error(b"bbb", "token_error"));
        let r: State<_, Vec<_>, _> = many1(new(DEFAULT, b"abb"), |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Data(new(DEFAULT, b"bb"), vec![b'a']));
        let r: State<_, Vec<_>, _> = many1(new(DEFAULT, b"aab"), |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Data(new(DEFAULT, b"b"), vec![b'a', b'a']));

        let r: State<_, Vec<_>, _> = many1(new(END_OF_INPUT, b""), |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Incomplete(1));
        let r: State<_, Vec<_>, _> = many1(new(END_OF_INPUT, b"a"), |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Data(new(END_OF_INPUT, b""), vec![b'a']));
        let r: State<_, Vec<_>, _> = many1(new(END_OF_INPUT, b"aa"), |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Data(new(END_OF_INPUT, b""), vec![b'a', b'a']));

        let r: State<_, Vec<_>, _> = many1(new(END_OF_INPUT, b"aab"), |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Data(new(END_OF_INPUT, b"b"), vec![b'a', b'a']));
    }

    #[test]
    fn count_test() {
        let r: State<_, Vec<_>, _> = count(new(DEFAULT, b""), 3,  |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Incomplete(1));
        let r: State<_, Vec<_>, _> = count(new(DEFAULT, b"a"), 3,  |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Incomplete(1));
        let r: State<_, Vec<_>, _> = count(new(DEFAULT, b"aa"), 3,  |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Incomplete(1));
        let r: State<_, Vec<_>, _> = count(new(DEFAULT, b"aaa"), 3,  |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Data(new(DEFAULT, b""), vec![b'a', b'a', b'a']));
        let r: State<_, Vec<_>, _> = count(new(DEFAULT, b"aaaa"), 3,  |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Data(new(DEFAULT, b"a"), vec![b'a', b'a', b'a']));

        let r: State<_, Vec<_>, _> = count(new(END_OF_INPUT, b""), 3,  |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Incomplete(1));
        let r: State<_, Vec<_>, _> = count(new(END_OF_INPUT, b"a"), 3,  |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Incomplete(1));
        let r: State<_, Vec<_>, _> = count(new(END_OF_INPUT, b"aa"), 3,  |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Incomplete(1));
        let r: State<_, Vec<_>, _> = count(new(END_OF_INPUT, b"aaa"), 3,  |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Data(new(END_OF_INPUT, b""), vec![b'a', b'a', b'a']));
        let r: State<_, Vec<_>, _> = count(new(END_OF_INPUT, b"aaaa"), 3,  |i| token(i, b'a')).into_inner();
        assert_eq!(r, State::Data(new(END_OF_INPUT, b"a"), vec![b'a', b'a', b'a']));
    }

    #[test]
    fn skip_many1_test() {
        assert_eq!(skip_many1(new(DEFAULT, b"aabc"), |i| token(i, b'a')).into_inner(), State::Data(new(DEFAULT, b"bc"), ()));
        assert_eq!(skip_many1(new(DEFAULT, b"abc"), |i| token(i, b'a')).into_inner(), State::Data(new(DEFAULT, b"bc"), ()));
        assert_eq!(skip_many1(new(DEFAULT, b"bc"), |i| i.err::<(), _>("error")).into_inner(), State::Error(b"bc", "error"));
        assert_eq!(skip_many1(new(DEFAULT, b"aaa"), |i| token(i, b'a')).into_inner(), State::Incomplete(1));
        assert_eq!(skip_many1(new(END_OF_INPUT, b"aabc"), |i| token(i, b'a')).into_inner(), State::Data(new(END_OF_INPUT, b"bc"), ()));
        assert_eq!(skip_many1(new(END_OF_INPUT, b"abc"), |i| token(i, b'a')).into_inner(), State::Data(new(END_OF_INPUT, b"bc"), ()));
        assert_eq!(skip_many1(new(END_OF_INPUT, b"bc"), |i| i.err::<(), _>("error")).into_inner(), State::Error(b"bc", "error"));
        assert_eq!(skip_many1(new(END_OF_INPUT, b"aaa"), |i| token(i, b'a')).into_inner(), State::Data(new(END_OF_INPUT, b""), ()));
    }

    #[test]
    fn many_till_test() {
        assert_eq!(many_till(new(DEFAULT, b"abcd"), any, |i| token(i, b'c')).into_inner(), State::Data(new(DEFAULT, b"d"), vec![b'a', b'b']));
        let r: ParseResult<_, Vec<_>, _> = many_till(new(DEFAULT, b"abd"), any, |i| token(i, b'c'));
        assert_eq!(r.into_inner(), State::Incomplete(1));

        let r: ParseResult<_, Vec<u8>, _> = many_till(new(DEFAULT, b"abcd"), |i| i.err(()), |i| token(i, b'c'));
        assert_eq!(r.into_inner(), State::Error(b"abcd", ()));

        // Variant to make sure error slice is propagated
        let mut n = 0;
        let r: ParseResult<_, Vec<_>, _> = many_till(new(DEFAULT, b"abcd"), |i| if n == 0 { n += 1; any(i).map_err(|_| "any err") } else { i.err("the error") }, |i| token(i, b'c'));
        assert_eq!(r.into_inner(), State::Error(b"bcd", "the error"));
    }

    #[test]
    fn matched_by_test() {
        assert_eq!(matched_by(new(DEFAULT, b"abc"), any).into_inner(), State::Data(new(DEFAULT, b"bc"), (&b"a"[..], b'a')));
        assert_eq!(matched_by(new(DEFAULT, b"abc"), |i| i.err::<(), _>("my error")).into_inner(), State::Error(&b"abc"[..], "my error"));
        assert_eq!(matched_by(new(DEFAULT, b"abc"), |i| any(i).map_err(|_| "any error").then(|i| i.err::<(), _>("my error"))).into_inner(), State::Error(&b"bc"[..], "my error"));
        assert_eq!(matched_by(new(DEFAULT, b""), any).into_inner(), State::Incomplete(1));
    }

    #[test]
    fn sep_by_test() {
        assert_eq!(sep_by(new(END_OF_INPUT, b""), any, |i| token(i, b';')).into_inner(), State::Data(new(END_OF_INPUT, b""), vec![]));
        assert_eq!(sep_by(new(END_OF_INPUT, b"b"), |i| token(i, b'a'), |i| token(i, b';')).into_inner(), State::Data(new(END_OF_INPUT, b"b"), vec![]));
        assert_eq!(sep_by(new(END_OF_INPUT, b"a"), any, |i| token(i, b';')).into_inner(), State::Data(new(END_OF_INPUT, b""), vec![b'a']));
        assert_eq!(sep_by(new(END_OF_INPUT, b"a;c"), any, |i| token(i, b';')).into_inner(), State::Data(new(END_OF_INPUT, b""), vec![b'a', b'c']));
        assert_eq!(sep_by(new(END_OF_INPUT, b"a;c;"), any, |i| token(i, b';')).into_inner(), State::Data(new(END_OF_INPUT, b";"), vec![b'a', b'c']));
        assert_eq!(sep_by(new(END_OF_INPUT, b"a--c-"), any, |i| string(i, b"--")).into_inner(), State::Data(new(END_OF_INPUT, b"-"), vec![b'a', b'c']));
        assert_eq!(sep_by(new(END_OF_INPUT, b"abc"), any, |i| token(i, b';')).into_inner(), State::Data(new(END_OF_INPUT, b"bc"), vec![b'a']));
        assert_eq!(sep_by(new(END_OF_INPUT, b"a;bc"), any, |i| token(i, b';')).into_inner(), State::Data(new(END_OF_INPUT, b"c"), vec![b'a', b'b']));

        assert_eq!(sep_by(new(DEFAULT, b"abc"), any, |i| token(i, b';')).into_inner(), State::Data(new(DEFAULT, b"bc"), vec![b'a']));
        assert_eq!(sep_by(new(DEFAULT, b"a;bc"), any, |i| token(i, b';')).into_inner(), State::Data(new(DEFAULT, b"c"), vec![b'a', b'b']));

        // Incomplete becasue there might be another separator or item to be read
        let r: ParseResult<_, Vec<_>, _> = sep_by(new(DEFAULT, b""), any, |i| token(i, b';'));
        assert_eq!(r.into_inner(), State::Incomplete(1));

        let r: ParseResult<_, Vec<_>, _> = sep_by(new(DEFAULT, b"a"), any, |i| token(i, b';'));
        assert_eq!(r.into_inner(), State::Incomplete(1));

        let r: ParseResult<_, Vec<_>, _> = sep_by(new(DEFAULT, b"a;"), any, |i| token(i, b';'));
        assert_eq!(r.into_inner(), State::Incomplete(1));

        let r: ParseResult<_, Vec<_>, _> = sep_by(new(DEFAULT, b"a;c"), any, |i| token(i, b';'));
        assert_eq!(r.into_inner(), State::Incomplete(1));

        let r: ParseResult<_, Vec<_>, _> = sep_by(new(DEFAULT, b"a;c;"), any, |i| token(i, b';'));
        assert_eq!(r.into_inner(), State::Incomplete(1));

        let r: ParseResult<_, Vec<_>, _> = sep_by(new(DEFAULT, b"a--c-"), any, |i| string(i, b"--"));
        assert_eq!(r.into_inner(), State::Incomplete(1));

        let r: ParseResult<_, Vec<_>, _> = sep_by(new(DEFAULT, b"aaa--a"), |i| string(i, b"aaa"), |i| string(i, b"--"));
        assert_eq!(r.into_inner(), State::Incomplete(2));

    }

    #[test]
    fn sep_by1_test() {
        let r: ParseResult<_, Vec<_>, _> = sep_by1(new(END_OF_INPUT, b""), any, |i| token(i, b';'));
        assert_eq!(r.into_inner(), State::Incomplete(1));

        let r: ParseResult<_, Vec<()>, _> = sep_by1(new(END_OF_INPUT, b"b"), |i| i.err("my err"), |i| token(i, b';').map_err(|_| "token_err"));
        assert_eq!(r.into_inner(), State::Error(b"b", "my err"));

        assert_eq!(sep_by1(new(END_OF_INPUT, b"a"), any, |i| token(i, b';')).into_inner(), State::Data(new(END_OF_INPUT, b""), vec![b'a']));
        assert_eq!(sep_by1(new(END_OF_INPUT, b"a;c"), any, |i| token(i, b';')).into_inner(), State::Data(new(END_OF_INPUT, b""), vec![b'a', b'c']));
        assert_eq!(sep_by1(new(END_OF_INPUT, b"a;c;"), any, |i| token(i, b';')).into_inner(), State::Data(new(END_OF_INPUT, b";"), vec![b'a', b'c']));
        assert_eq!(sep_by1(new(END_OF_INPUT, b"a--c-"), any, |i| string(i, b"--")).into_inner(), State::Data(new(END_OF_INPUT, b"-"), vec![b'a', b'c']));
        assert_eq!(sep_by1(new(END_OF_INPUT, b"abc"), any, |i| token(i, b';')).into_inner(), State::Data(new(END_OF_INPUT, b"bc"), vec![b'a']));
        assert_eq!(sep_by1(new(END_OF_INPUT, b"a;bc"), any, |i| token(i, b';')).into_inner(), State::Data(new(END_OF_INPUT, b"c"), vec![b'a', b'b']));

        assert_eq!(sep_by1(new(DEFAULT, b"abc"), any, |i| token(i, b';')).into_inner(), State::Data(new(DEFAULT, b"bc"), vec![b'a']));
        assert_eq!(sep_by1(new(DEFAULT, b"a;bc"), any, |i| token(i, b';')).into_inner(), State::Data(new(DEFAULT, b"c"), vec![b'a', b'b']));

        // Incomplete becasue there might be another separator or item to be read
        let r: ParseResult<_, Vec<_>, _> = sep_by1(new(DEFAULT, b""), any, |i| token(i, b';'));
        assert_eq!(r.into_inner(), State::Incomplete(1));

        let r: ParseResult<_, Vec<_>, _> = sep_by1(new(DEFAULT, b"a"), any, |i| token(i, b';'));
        assert_eq!(r.into_inner(), State::Incomplete(1));

        let r: ParseResult<_, Vec<_>, _> = sep_by1(new(DEFAULT, b"a;"), any, |i| token(i, b';'));
        assert_eq!(r.into_inner(), State::Incomplete(1));

        let r: ParseResult<_, Vec<_>, _> = sep_by1(new(DEFAULT, b"a;c"), any, |i| token(i, b';'));
        assert_eq!(r.into_inner(), State::Incomplete(1));

        let r: ParseResult<_, Vec<_>, _> = sep_by1(new(DEFAULT, b"a;c;"), any, |i| token(i, b';'));
        assert_eq!(r.into_inner(), State::Incomplete(1));

        let r: ParseResult<_, Vec<_>, _> = sep_by1(new(DEFAULT, b"a--c-"), any, |i| string(i, b"--"));
        assert_eq!(r.into_inner(), State::Incomplete(1));

        let r: ParseResult<_, Vec<_>, _> = sep_by1(new(DEFAULT, b"aaa--a"), |i| string(i, b"aaa"), |i| string(i, b"--"));
        assert_eq!(r.into_inner(), State::Incomplete(2));
    }

    #[test]
    fn look_ahead_test() {
        assert_eq!(look_ahead(new(DEFAULT, b"abc"), any).into_inner(), State::Data(new(DEFAULT, b"abc"), b'a'));
        assert_eq!(look_ahead(new(DEFAULT, b"a"), |i| string(i, b"abc")).into_inner(), State::Incomplete(2));
        assert_eq!(look_ahead(new(DEFAULT, b"aa"), |i| token(i, b'a').then(|i| token(i, b'b')).map_err(|_| "err")).into_inner(), State::Error(b"a", "err"));
    }
}