pub(crate) fn print_usage() {
    eprintln!(
        r#"ergokeygen - ergonomic keyboard wordlist generator

USAGE:
  ergokeygen gen [options]
  ergokeygen score <sequence> [options]
  ergokeygen compare <left> <right> [options]
  ergokeygen layout

GENERATE OPTIONS:
  --min, --min-len N             Minimum length, default 4
  --max, --max-len N             Maximum length, default 8
  --limit N                      Maximum sequences to print; default is unlimited
  --beam N                       Beam width, default 1000000
  --jobs, --threads N            Rayon worker threads; default uses Rayon/CPU default
  --engine ENGINE                reference|fast-v1, default fast-v1
  --reference                    Use the conservative Rust contract engine
  --fast-v1                      Use the first accelerated engine, default
  --parallel                     Enable parallel beam expansion, default
  --single-thread, --no-parallel Reference single-thread expansion
  --parallel-threshold N         Minimum expansion work before Rayon, default 8192
  --chars NAME_OR_CHARS          lower|upper|letters|digits|lowerdigits|symbols|full|custom chars
  --mode MODE                    onehand|neutral|alternation, default onehand
  --prefer-hand HAND             any|left|right, default any
  --max-avg-cost X               Drop sequences above average score
  --max-total-cost X             Drop sequences above total score
  --show-score                   Print average, total, sequence
  --stream                       Emit each completed length immediately; lower latency, relaxed global order
  --dedupe [fast|exact|off]      Optional output dedupe; bare --dedupe means fast
  --dedupe-mode MODE             Same as --dedupe MODE; useful for explicit scripts
  --dedupe-exact                 Exact HashSet<String> dedupe; slower, lossless
  --no-dedupe                    Disable output dedupe, default
  --config PATH                  Optional ergonomic model config

SCORE / COMPARE OPTIONS:
  --mode MODE                    onehand|neutral|alternation
  --prefer-hand HAND             any|left|right
  --config PATH                  Optional ergonomic model config
  --explain                      Print per-character scoring breakdown for score

EXAMPLES:
  ergokeygen gen --min 4 --max 8 --prefer-hand left --mode onehand --chars lower
  ergokeygen gen --min 4 --max 8 --beam 10000000 --prefer-hand left --mode onehand --chars lower
  ergokeygen gen --stream --min 4 --max 8 --limit 99999 --prefer-hand left --mode onehand --chars lower | hashcat -a 0 ...
  ergokeygen score asdf --prefer-hand left --mode onehand --explain
  ergokeygen compare asdf fddf --prefer-hand left --mode onehand

BEAM TUNING:
  --beam is usually the main knob to change. Higher values keep more candidate
  prefixes at each length, improving coverage and output size at the cost of
  RAM and CPU. Lower values run faster and use less memory, but prune harder.
"#
    );
}
