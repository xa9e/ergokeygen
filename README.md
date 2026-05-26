# Ergokeygen

**Ergokeygen** is an ergonomic keyboard wordlist generator. It generates keyboard sequences not only by geometric adjacency, but by an approximate model of how a human hand moves while typing.

The project started from a simple problem: tools such as keyboard-walk generators can produce geometric patterns, but they do not really answer the question: **how hard is the next key to press from the current hand state?** Ergokeygen tries to answer that question.

It is designed for high-volume wordlist generation where the production path must be fast enough to pipe into tools such as hashcat. The current Rust implementation is treated as the reference contract for future faster engines.

## Example output

For lowercase left-hand one-hand generation, the current model starts with
strings that are easy to type as smooth left-hand rolls:

```bash
ergokeygen gen --min 4 --max 8 --prefer-hand left --mode onehand --chars lower | head
```

```text
asdf
asdfasdf
sdfasdf
asdfsdf
fasdf
sdfsdf
fewasdf
dfasdf
asdfasd
sefasdf
```

If you ask only for 4-character strings, the top candidates are:

```bash
ergokeygen gen --min 4 --max 4 --prefer-hand left --mode onehand --chars lower | head
```

```text
asdf
fsdf
fdsa
fewa
asef
qsdf
fasd
sdfs
fsef
sefd
```

At the other end of a very broad 4-character run, the model ranks strings like
these as uncomfortable for left-hand one-hand typing:

```bash
ergokeygen gen --min 4 --max 4 --prefer-hand left --mode onehand --chars lower --beam 10000000 | tail
```

```text
myny
ymny
nmym
ymmy
ynym
nymy
ymyn
mymy
ymym
pppp
```

## Why this is unusual

Most keyboard-pattern generators treat the keyboard as a grid. Ergokeygen treats typing as a motor sequence.

It models:

- relaxed left-hand finger positions, not only key coordinates;
- dynamic finger positions after each keypress;
- palm drift and row bias after moving to upper or lower rows;
- timing-based finger recovery, so `FD` is faster than `FF`, but `ASDFASDF` does not get punished as if every repeated finger was immediately reused;
- smooth rolls and sweeps such as `asdf`, `qwer`, `zxcv`;
- direction continuity before sweeps, so `DF+ASDF` is preferred over `FD+ASDF` because the prefix and the sweep move in the same physical direction;
- mixed motor programs such as `awdf`, where one finger changes row while the others do not;
- natural finger-axis deviation, so `asef` is easier than `awdf` because middle-to-`E` is closer to straight finger extension, while ring-to-`W` requires lateral correction;
- stretch keys such as `T`, `G`, `B`, half-stretch `V`, and left-hand number-row reaches to `5`/`6`;
- weak cognitive pattern likelihood, deliberately capped so obvious password-dictionary patterns do not dominate the physical model;
- optional profile files for tuning the model without recompiling.

I would not honestly claim that no related tool exists anywhere. The more defensible claim is: Ergokeygen is not just another keyboard-walk enumerator. Its core value is the combination of ergonomic scoring, timing, hand posture, weak pattern likelihood, and a Rust generator that can later grow multiple optimized engines behind the same contract.

## Current model

Ergokeygen scores a candidate by combining several components.

Static cost:

- key row difficulty;
- distance from relaxed finger position;
- pinky/ring weakness;
- Shift penalty;
- left-index stretch penalties;
- natural finger-axis lateral deviation.

Dynamic cost:

- current finger-tip position;
- current palm posture;
- accumulated palm tension.

Transition and rhythm cost:

- adjacent-finger rolls;
- forward sweeps;
- reverse sweeps;
- same-finger movement;
- ABBA-style bounce;
- redirects;
- pair-direction changes before a sweep-like motor program;
- same-hand or cross-hand rhythm depending on mode.

Timing cost:

- every finger has a `ready_at` timestamp;
- a keypress waits if the requested finger is not ready;
- repeated use after a 3-4 finger pipeline is much cheaper than immediate repeat.

Cognitive likelihood:

- known keyboard shapes get a small bonus;
- the bonus is capped;
- this component is intentionally weak because common dictionaries already contain obvious strings such as `qwerty`, `asdf`, and `1234`.

## Quick start

On Arch Linux:

```bash
sudo pacman -S --needed rust
unzip ergokeygen-v11.zip
cd ergokeygen
cargo build --release
cargo test
```

Generate lowercase left-hand ergonomic candidates:

```bash
./target/release/ergokeygen gen \
  --min 4 \
  --max 8 \
  --prefer-hand left \
  --mode onehand \
  --chars lower
```

By default `gen` prints every candidate retained by the configured beam and
length range. Add `--limit N` when you want only the top `N` lines.

The main parameter you will usually tune is `--beam`. A larger beam keeps more
candidate prefixes at every length, so coverage and output size go up, while
CPU and memory use also increase. A smaller beam runs faster and uses less
memory, but prunes the search more aggressively. The default beam is `1000000`.

On one test machine, this mode produced about 4.46 million lines in about 2.2
seconds:

```bash
time ./target/release/ergokeygen gen \
  --min 4 \
  --max 8 \
  --beam 1000000 \
  --prefer-hand left \
  --mode onehand \
  --chars lower | wc -l
```

Example output:

```text
4456976
./target/release/ergokeygen gen ...  19.85s user 0.98s system 949% cpu 2.194 total
wc -l  0.00s user 0.01s system 0% cpu 2.194 total
```

For a broader demo run, try `--beam 10000000`. For quick experiments or
low-memory machines, reduce `--beam` substantially.

Pipe into hashcat:

```bash
./target/release/ergokeygen gen --min 4 --max 8 --prefer-hand left --mode onehand --chars lower \
  | hashcat -a 0 -m 1000 hashes.txt --stdin
```

For low-latency pipelines, use `--stream`:

```bash
./target/release/ergokeygen gen --stream --min 4 --max 8 --limit 999999 --prefer-hand left --mode onehand --chars lower \
  | hashcat -a 0 -m 1000 hashes.txt --stdin
```

The normal generator keeps exact global ordering across lengths. `--stream` emits each completed length immediately, so the first batch reaches `less`, `wc`, or `hashcat` earlier. The ergonomic scoring and beam contents are the same, but global cross-length ordering is relaxed.

Optional output dedupe is available, but it is disabled by default:

```bash
./target/release/ergokeygen gen --dedupe fast --stream --min 4 --max 12 --limit 1000000 --prefer-hand left --mode onehand --chars lower
./target/release/ergokeygen gen --dedupe exact --min 4 --max 12 --limit 100000 --prefer-hand left --mode onehand --chars lower
```

Current single-beam generation usually emits unique strings because the charset is deduplicated before search. `--dedupe fast` exists for custom charsets and future multi-family generation. It stores a 64-bit non-cryptographic fingerprint and may theoretically skip a candidate on collision. `--dedupe exact` stores full strings and is lossless but slower and more memory-hungry.

Score one sequence:

```bash
./target/release/ergokeygen score asdf --prefer-hand left --mode onehand --explain
```

Compare two sequences:

```bash
./target/release/ergokeygen compare asef awdf --prefer-hand left --mode onehand
```

Use an optional profile:

```bash
./target/release/ergokeygen gen \
  --config profiles/left-ring-strict.ekg \
  --min 4 \
  --max 8 \
  --limit 10000 \
  --prefer-hand left \
  --mode onehand \
  --chars lower
```

## Performance notes

Generation used to build and globally sort all candidates before printing anything. That made short runs feel like they had a startup stall. The Rust generator now uses:

- a compact beam-search state that does not clone `score --explain` histories;
- pre-resolved `Key` objects for the charset;
- `select_nth_unstable_by` to keep the best beam window without fully sorting every expanded candidate set;
- one final global sort in normal mode instead of repeated accumulated sorts;
- parallel beam expansion with Rayon;
- `BufWriter` for stdout;
- optional `--stream` mode for early output to pipes;
- optional output dedupe with `off`, `fast`, and `exact` modes.

The default generation path uses `--engine fast-v1`, which expands large frontiers with Rayon when that is profitable. `--engine reference` / `--reference` keeps the conservative Rust contract path. Both engines must keep the same scoring and final ordering because every depth is still merged, retained, and sorted through one deterministic comparator. Use `--single-thread` when debugging the fast engine without Rayon. Use `--jobs N` / `--threads N` to cap CPU usage. `--parallel-threshold N` controls when Rayon starts being used; very small frontiers are cheaper to expand sequentially.

Examples:

```bash
./target/release/ergokeygen gen --jobs 16 --beam 60000 --min 4 --max 20 --limit 1000000 --prefer-hand left --mode onehand --chars lower
./target/release/ergokeygen gen --reference --beam 4096 --min 4 --max 8 --limit 10000 --prefer-hand left --mode onehand --chars lower
./target/release/ergokeygen gen --single-thread --beam 4096 --min 4 --max 8 --limit 10000 --prefer-hand left --mode onehand --chars lower
```

Use normal mode when exact global ordering across lengths matters. Use `--stream` when feeding another process and first-line latency matters more than exact cross-length ordering. `--stream` relaxes only cross-length ordering; the beam expansion itself can still be parallel.

## Rust contract tests

The current Rust implementation is the contract for optimized engines. Contract tests snapshot key scores, top generated candidates, CLI output, and parity between `reference` and `fast-v1`.

Run tests:

```bash
cargo test
```

## Optional configuration

The binary works without any config file. Defaults are hardcoded in Rust. A config file only overrides selected values.

Example:

```ini
[weights]
finger_axis_lateral = 0.95
mixed_motor_program_penalty = 1.00

[finger.left_ring]
lateral_factor = 1.70
axis_dx = 0.32
axis_dy = -1.00
```

Use `profiles/default.ekg` as a fully commented baseline. Use `profiles/left-ring-strict.ekg` as an example of making `AWDF`-like patterns more expensive.

Detailed config documentation is in `docs/CONFIG.md`.

## Repository layout

```text
ergokeygen/
  Cargo.toml
  README.md
  IDEA.md
  profiles/
    default.ekg
    left-ring-strict.ekg
  docs/
    CONFIG.md
    RESEARCH.md
  src/
    lib.rs
    main.rs
  tests/
    contract.rs
```

## Development rules

The intended workflow:

- document the idea in `IDEA.md`;
- add regression and contract tests for observed ergonomic preferences;
- keep `--engine reference` as the conservative Rust contract;
- add optimized engines as separately selectable versions such as `fast-v1`;
- keep defaults hardcoded;
- keep config files optional;
- document research-backed constants near the implementation and in `docs/RESEARCH.md`.

## Current limitations

The model is still approximate.

Known missing or incomplete parts:

- no full 3D hand skeleton yet;
- no real joint-angle solver;
- Shift posture is still simplified;
- right-hand calibration is weaker than left-hand calibration;
- generator diversity is still beam-search-centric;
- seed-based mutation is not implemented yet;
- no automatic pairwise weight tuner yet.

The immediate next major step is a better hand model: not full medical simulation, but a configurable lightweight model of finger axes, lateral stiffness, coupling, palm posture, and timing.

## Responsible Use

Use Ergokeygen only for systems and data you own or are explicitly authorized to test. The tool is intended for ergonomic sequence research, password-audit preparation, and defensive security workflows. Do not use it to access accounts, systems, or data without permission.

---

# Ergokeygen

**Ergokeygen** — генератор эргономичных клавиатурных wordlist-последовательностей. Он генерирует строки не только по геометрической близости клавиш, а по приближённой модели того, как человеческая рука двигается при печати.

Проект вырос из простой проблемы: тулзы наподобие генераторов keyboard-walk умеют выдавать геометрические паттерны, но не отвечают по-настоящему на вопрос: **насколько тяжело нажать следующую клавишу из текущего состояния руки?** Ergokeygen пытается отвечать именно на этот вопрос.

Он рассчитан на высокообъёмную генерацию wordlist-ов, где production-путь должен быть достаточно быстрым, чтобы кормить инструменты вроде hashcat через pipe. Текущая Rust-реализация считается reference-контрактом для будущих более быстрых engine-ов.

## Пример вывода

Для lowercase one-hand генерации под левую руку текущая модель начинает со
строк, которые удобно печатать как плавные left-hand rolls:

```bash
ergokeygen gen --min 4 --max 8 --prefer-hand left --mode onehand --chars lower | head
```

```text
asdf
asdfasdf
sdfasdf
asdfsdf
fasdf
sdfsdf
fewasdf
dfasdf
asdfasd
sefasdf
```

Если запросить только 4-символьные строки, верх выдачи такой:

```bash
ergokeygen gen --min 4 --max 4 --prefer-hand left --mode onehand --chars lower | head
```

```text
asdf
fsdf
fdsa
fewa
asef
qsdf
fasd
sdfs
fsef
sefd
```

На другом конце очень широкого 4-символьного запуска текущая модель считает
примерно такие строки неудобными для печати одной левой рукой:

```bash
ergokeygen gen --min 4 --max 4 --prefer-hand left --mode onehand --chars lower --beam 10000000 | tail
```

```text
myny
ymny
nmym
ymmy
ynym
nymy
ymyn
mymy
ymym
pppp
```

## Почему это необычно

Большинство генераторов клавиатурных паттернов воспринимают клавиатуру как сетку. Ergokeygen воспринимает печать как моторную последовательность.

Он моделирует:

- расслабленные позиции пальцев левой руки, а не только координаты клавиш;
- динамические позиции пальцев после каждого нажатия;
- смещение ладони и bias ряда после перехода на верхний или нижний ряд;
- временное восстановление пальца, поэтому `FD` быстрее, чем `FF`, но `ASDFASDF` не штрафуется так, будто каждый повтор пальца происходит сразу;
- плавные rolls и sweeps вроде `asdf`, `qwer`, `zxcv`;
- смешанные моторные программы вроде `awdf`, где один палец меняет ряд, а остальные нет;
- отклонение от естественной оси пальца, поэтому `asef` легче, чем `awdf`: средний палец к `E` идёт ближе к прямому разгибанию, а безымянный к `W` требует боковой коррекции;
- stretch-клавиши вроде `T`, `G`, `B`, half-stretch `V` и левые дотягивания на цифровом ряду к `5`/`6`;
- слабую cognitive pattern likelihood, специально ограниченную, чтобы очевидные словарные парольные паттерны не доминировали над физической моделью;
- опциональные profile-файлы для настройки модели без перекомпиляции.

Я бы не стал честно утверждать, что нигде не существует ничего похожего. Более корректное утверждение: Ergokeygen — это не очередной перечислитель keyboard-walk. Его ценность в сочетании эргономичного scoring, timing, hand posture, слабой pattern likelihood и Rust-генератора, который может развивать несколько оптимизированных engine-ов под одним контрактом.

## Текущая модель

Ergokeygen оценивает кандидата через несколько компонентов.

Статическая стоимость:

- сложность ряда клавиши;
- расстояние от расслабленной позиции пальца;
- слабость мизинца/безымянного;
- штраф за Shift;
- штрафы за stretch левого указательного;
- боковое отклонение от естественной оси пальца.

Динамическая стоимость:

- текущая позиция кончика пальца;
- текущая поза ладони;
- накопленное напряжение ладони.

Стоимость переходов и ритма:

- rolls соседними пальцами;
- forward sweeps;
- reverse sweeps;
- движение тем же пальцем;
- ABBA-bounce;
- redirects;
- same-hand или cross-hand ритм в зависимости от режима.

Временная стоимость:

- у каждого пальца есть timestamp `ready_at`;
- нажатие ждёт, если нужный палец ещё не готов;
- повтор после конвейера из 3-4 пальцев намного дешевле, чем мгновенный повтор.

Cognitive likelihood:

- известные клавиатурные формы получают маленький бонус;
- бонус ограничен сверху;
- компонент специально слабый, потому что общие словари уже содержат очевидные строки вроде `qwerty`, `asdf` и `1234`.

## Быстрый старт

На Arch Linux:

```bash
sudo pacman -S --needed rust
unzip ergokeygen-v11.zip
cd ergokeygen
cargo build --release
cargo test
```

Генерация lowercase-кандидатов под левую руку:

```bash
./target/release/ergokeygen gen \
  --min 4 \
  --max 8 \
  --prefer-hand left \
  --mode onehand \
  --chars lower
```

По умолчанию `gen` печатает все кандидаты, оставшиеся после заданного beam и
диапазона длин. Добавь `--limit N`, если нужны только первые `N` строк.

Главный параметр, который обычно имеет смысл менять, это `--beam`. Чем выше
beam, тем больше candidate-prefixes сохраняется на каждой длине, тем шире
coverage и больше итоговый вывод, но тем выше расход CPU и памяти. Чем ниже
beam, тем быстрее и экономнее запуск, но тем агрессивнее обрезается поиск.
Дефолтный beam: `1000000`.

На одной тестовой машине такой режим сгенерировал около 4.46 млн строк примерно
за 2.2 секунды:

```bash
time ./target/release/ergokeygen gen \
  --min 4 \
  --max 8 \
  --beam 1000000 \
  --prefer-hand left \
  --mode onehand \
  --chars lower | wc -l
```

Пример вывода:

```text
4456976
./target/release/ergokeygen gen ...  19.85s user 0.98s system 949% cpu 2.194 total
wc -l  0.00s user 0.01s system 0% cpu 2.194 total
```

Для более широкой демки можно поставить `--beam 10000000`. Для быстрых проб или
машин с небольшим запасом RAM, наоборот, сильно уменьши `--beam`.

Pipe в hashcat:

```bash
./target/release/ergokeygen gen --min 4 --max 8 --prefer-hand left --mode onehand --chars lower \
  | hashcat -a 0 -m 1000 hashes.txt --stdin
```

Для pipeline с минимальной задержкой первого вывода используй `--stream`:

```bash
./target/release/ergokeygen gen --stream --min 4 --max 8 --limit 999999 --prefer-hand left --mode onehand --chars lower \
  | hashcat -a 0 -m 1000 hashes.txt --stdin
```

Обычный генератор сохраняет точную глобальную сортировку между длинами. `--stream` выводит каждую завершённую длину сразу, поэтому первый блок строк быстрее доходит до `less`, `wc` или `hashcat`. Эргономическая оценка и beam остаются теми же, но глобальный порядок между разными длинами становится менее строгим.

Опциональная дедупликация вывода есть, но по умолчанию выключена:

```bash
./target/release/ergokeygen gen --dedupe fast --stream --min 4 --max 12 --limit 1000000 --prefer-hand left --mode onehand --chars lower
./target/release/ergokeygen gen --dedupe exact --min 4 --max 12 --limit 100000 --prefer-hand left --mode onehand --chars lower
```

Текущий single-beam генератор обычно и так выдаёт уникальные строки, потому что charset дедуплицируется до поиска. `--dedupe fast` нужен для custom charset-ов и будущей генерации через несколько семейств паттернов. Он хранит 64-битный некриптографический fingerprint и теоретически может пропустить кандидата при collision. `--dedupe exact` хранит полные строки, не даёт ложных совпадений, но медленнее и прожорливее по памяти.

Оценить одну последовательность:

```bash
./target/release/ergokeygen score asdf --prefer-hand left --mode onehand --explain
```

Сравнить две последовательности:

```bash
./target/release/ergokeygen compare asef awdf --prefer-hand left --mode onehand
```

Использовать опциональный профиль:

```bash
./target/release/ergokeygen gen \
  --config profiles/left-ring-strict.ekg \
  --min 4 \
  --max 8 \
  --limit 10000 \
  --prefer-hand left \
  --mode onehand \
  --chars lower
```

## Заметки по производительности

Раньше генератор строил и глобально сортировал все кандидаты до первого вывода. Из-за этого короткие запуски выглядели так, будто в начале есть простой. Rust-генератор теперь использует:

- компактное состояние beam search, которое не клонирует историю `score --explain`;
- заранее подготовленные `Key`-объекты для charset;
- `select_nth_unstable_by`, чтобы оставлять лучший beam-window без полной сортировки всех expanded candidates;
- одну финальную глобальную сортировку в обычном режиме вместо повторных сортировок накопленного результата;
- параллельное расширение beam через Rayon;
- `BufWriter` для stdout;
- опциональный режим `--stream` для раннего вывода в pipe;
- опциональную дедупликацию вывода в режимах `off`, `fast`, `exact`.

Дефолтный путь генерации использует `--engine fast-v1`, который расширяет большие frontier-ы через Rayon, когда это выгодно. `--engine reference` / `--reference` включает консервативный Rust-путь, который служит контрактом. Оба engine-а должны сохранять тот же scoring и финальный порядок, потому что каждый depth всё равно сливается, обрезается и сортируется одним детерминированным comparator-ом. Используй `--single-thread` для отладки fast engine без Rayon. Используй `--jobs N` / `--threads N`, чтобы ограничить число worker threads. `--parallel-threshold N` управляет моментом, когда начинает использоваться Rayon; маленькие frontier-ы дешевле расширять последовательно.

Примеры:

```bash
./target/release/ergokeygen gen --jobs 16 --beam 60000 --min 4 --max 20 --limit 1000000 --prefer-hand left --mode onehand --chars lower
./target/release/ergokeygen gen --reference --beam 4096 --min 4 --max 8 --limit 10000 --prefer-hand left --mode onehand --chars lower
./target/release/ergokeygen gen --single-thread --beam 4096 --min 4 --max 8 --limit 10000 --prefer-hand left --mode onehand --chars lower
```

Обычный режим лучше, когда важна точная глобальная сортировка между длинами. `--stream` лучше для pipeline в другой процесс, когда задержка первого вывода важнее строгого cross-length порядка. `--stream` расслабляет только cross-length ordering; само расширение beam всё равно может быть параллельным.

## Rust contract tests

Текущая Rust-реализация является контрактом для оптимизированных engine-ов. Contract-тесты фиксируют ключевые score-ы, верхние generated candidates, CLI output и parity между `reference` и `fast-v1`.

Запуск тестов:

```bash
cargo test
```

## Опциональная конфигурация

Бинарник работает без config-файла. Defaults захардкожены в Rust. Config-файл только переопределяет выбранные значения.

Пример:

```ini
[weights]
finger_axis_lateral = 0.95
mixed_motor_program_penalty = 1.00

[finger.left_ring]
lateral_factor = 1.70
axis_dx = 0.32
axis_dy = -1.00
```

Используй `profiles/default.ekg` как полностью прокомментированный baseline. Используй `profiles/left-ring-strict.ekg` как пример того, как сделать `AWDF`-подобные паттерны дороже.

Подробная документация по конфигу лежит в `docs/CONFIG.md`.

## Структура репозитория

```text
ergokeygen/
  Cargo.toml
  README.md
  IDEA.md
  profiles/
    default.ekg
    left-ring-strict.ekg
  docs/
    CONFIG.md
    RESEARCH.md
  src/
    lib.rs
    main.rs
  tests/
    contract.rs
```

## Правила разработки

Предполагаемый workflow:

- документировать идею в `IDEA.md`;
- добавлять regression и contract tests для наблюдаемых эргономических предпочтений;
- держать `--engine reference` как консервативный Rust-контракт;
- добавлять оптимизированные engine-ы отдельными версиями вроде `fast-v1`;
- держать defaults захардкоженными;
- держать config-файлы опциональными;
- документировать research-backed константы рядом с реализацией и в `docs/RESEARCH.md`.

## Текущие ограничения

Модель всё ещё приближённая.

Известные недоделки:

- пока нет полноценного 3D-скелета кисти;
- нет настоящего solver-а углов суставов;
- Shift posture всё ещё упрощён;
- правая рука откалибрована хуже левой;
- разнообразие генератора всё ещё завязано на beam search;
- seed-based mutation пока не реализован;
- автоматический pairwise weight tuner пока не реализован.

Следующий крупный шаг — лучшая модель кисти: не полноценная медицинская симуляция, а конфигурируемая лёгкая модель осей пальцев, боковой жёсткости, связанности пальцев, позы ладони и timing.

## Ответственное использование

Используй Ergokeygen только для систем и данных, которыми ты владеешь или которые явно разрешено тестировать. Инструмент предназначен для исследования эргономичных последовательностей, подготовки password-аудита и защитных security workflow. Не используй его для доступа к аккаунтам, системам или данным без разрешения.

## FEW(A/Q) coupled reverse clusters / FEW(A/Q) как связанный reverse-кластер

`FEWQ` and `FEWA` are treated as FDSA-like split reverse sweeps, not as generic mixed-row noise. The model gives context-sensitive relief to `E -> W`: after the middle finger extends to `E`, the ring finger is mechanically closer to `W`. This keeps `AWDF` expensive while allowing useful patterns such as `fewa`, `fewq`, and `fewas` to appear earlier.

`FEWQ` и `FEWA` считаются split reverse sweep, близким к `FDSA`, а не обычным mixed-row мусором. Модель даёт контекстное облегчение для `E -> W`: после разгибания среднего пальца к `E` безымянный механически оказывается ближе к `W`. Поэтому `AWDF` остаётся дорогим, но полезные паттерны `fewa`, `fewq`, `fewas` появляются раньше.
