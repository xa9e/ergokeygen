# Research notes

This file documents why several constants and model branches exist. It is not a medical claim that Ergokeygen simulates a real hand exactly. It is a record of design constraints and sources so future maintainers do not have to rediscover the same reasoning.

## User observations that drive the model

Important empirical observations from the project discussion:

- In left-hand one-hand mode, `asdf` should be the first or near-first result because it is a smooth left-to-right home-row sweep with four adjacent fingers.
- `fddf` and `dffd` are too bounce-heavy to appear early.
- `FD` should be faster than `FF`, but `ASDFASDF` should not be heavily punished: the finger has time to recover through the other three keypresses.
- `AWDF` is much harder than `ASEF`: middle-to-`E` is close to straight finger extension, but ring-to-`W` needs a lateral correction to avoid also pressing neighbouring keys.
- After `zxcv`, the hand/palm is biased toward the bottom row; continuing there is easier than jumping to the top row.
- Shift in left-hand typing occupies the pinky and shifts the hand left, making rightward reaches more expensive.
- Cognitive pattern likelihood matters, but must stay weak because obvious password patterns already exist in common dictionaries.

## Anatomy/biomechanics references used as qualitative support

These sources are used only as qualitative guidance for the heuristic model. They should not be read as proof that the current coefficients are medically precise.

Reference URLs:

- NCBI Bookshelf / StatPearls, hand anatomy and interossei: https://www.ncbi.nlm.nih.gov/sites/books/NBK537165/
- RSNA extensor mechanism anatomy review: https://pubs.rsna.org/doi/10.1148/rg.233025079
- Schieber lab material on finger independence constraints: https://www.urmc.rochester.edu/MediaLibraries/URMCMedia/labs/schieber-lab/documents/2003_SFN_2003_CEL_poster3_011604.pdf
- Scientific Reports paper on finger dexterity in musicians: https://www.nature.com/articles/s41598-019-48718-9
- Vergara et al. hand anthropometry paper mirror: https://core.ac.uk/download/pdf/141439994.pdf

Summary:

- NCBI Bookshelf, StatPearls: intrinsic hand muscles and interossei abduct/adduct fingers and support MCP/IP mechanics. This supports an explicit lateral-deviation penalty instead of treating every upper-row reach as simple forward extension.
- RSNA review of extensor mechanism anatomy: finger extension is an interaction of intrinsic/extrinsic muscles and retinacular structures, not independent one-actuator-per-finger mechanics. This supports coupling and mixed motor-program penalties.
- Schieber lab material on passive and active finger independence constraints: fingers are mechanically and neurologically coupled. This supports avoiding independent-point-mass finger modelling.
- Scientific Reports paper on finger dexterity in musicians: tapping rate and finger independence depend on neuromuscular and biomechanical constraints. This supports timing/recovery modelling instead of a static same-finger penalty.
- Vergara et al. hand anthropometry work: detailed hand dimensions matter for ergonomic modelling. This supports future configurable hand-profile work instead of pretending one universal hand exists.

## Where this is represented in code

- `finger_axis_deviation`: off-axis lateral correction. Motivation: `ASEF < AWDF`.
- `finger_ready` / `finger_recovery_time`: timing-based bounce model. Motivation: `FD < FF`, but `ASDFA` recovers enough.
- `uniform_motor_program` / `mixed_motor_program_mismatch`: repeated block uniformity. Motivation: `ASDFASDF < AWDFAWDF`.
- `PalmPosture`: rough hand row-bias/tension model. Motivation: `zxcv` creates a lower-row posture.
- `cognitive_pattern_adjustment`: weak salience bonus for known keyboard shapes. Motivation: humans choose patterns, but obvious ones should not dominate because dictionaries already cover them.
- `load_weights_config`: sparse override layer for personal calibration, keeping defaults hardcoded.

## Future research debt

The current model is still 2D/2.5D. Future work may add:

- configurable hand anthropometry profile;
- finger coupling matrix;
- better Shift posture with occupied pinky;
- pairwise tuning from observations such as `asef < awdf`;
- optional 3D kinematic approximation if it gives measurable ranking gains.

---

# Заметки по исследованию

Этот файл объясняет, почему в модели есть некоторые константы и ветки. Это не медицинское утверждение, что Ergokeygen точно симулирует реальную кисть. Это журнал проектных ограничений и источников, чтобы будущим maintainer-ам и агентам не приходилось заново восстанавливать ту же логику.

## Наблюдения пользователя, на которых держится модель

Важные эмпирические наблюдения из обсуждения проекта:

- В left-hand one-hand режиме `asdf` должен быть первым или около первого результата, потому что это плавный left-to-right home-row sweep четырьмя соседними пальцами.
- `fddf` и `dffd` слишком bounce-heavy, чтобы появляться рано.
- `FD` должен быть быстрее, чем `FF`, но `ASDFASDF` не должен сильно штрафоваться: палец успевает восстановиться за три других нажатия.
- `AWDF` значительно тяжелее, чем `ASEF`: средний к `E` идёт близко к прямому разгибанию пальца, а безымянный к `W` требует боковой коррекции, чтобы не задеть соседние клавиши.
- После `zxcv` кисть/ладонь смещается к нижнему ряду; продолжать там легче, чем прыгать на верхний ряд.
- Shift при наборе левой рукой занимает мизинец и смещает руку влево, делая правые дотягивания дороже.
- Cognitive pattern likelihood важен, но должен быть слабым, потому что очевидные парольные паттерны уже есть в общих словарях.

## Анатомические/биомеханические источники как качественная поддержка

Эти источники используются только как качественная опора для эвристической модели. Их нельзя читать как доказательство медицинской точности текущих коэффициентов.

URL источников:

- NCBI Bookshelf / StatPearls, hand anatomy and interossei: https://www.ncbi.nlm.nih.gov/sites/books/NBK537165/
- RSNA extensor mechanism anatomy review: https://pubs.rsna.org/doi/10.1148/rg.233025079
- Материалы Schieber lab по constraints независимости пальцев: https://www.urmc.rochester.edu/MediaLibraries/URMCMedia/labs/schieber-lab/documents/2003_SFN_2003_CEL_poster3_011604.pdf
- Scientific Reports paper про finger dexterity у музыкантов: https://www.nature.com/articles/s41598-019-48718-9
- Vergara et al. hand anthropometry paper mirror: https://core.ac.uk/download/pdf/141439994.pdf

Кратко:

- NCBI Bookshelf, StatPearls: intrinsic muscles кисти и interossei abduct/adduct пальцы и участвуют в MCP/IP механике. Это поддерживает отдельный штраф за lateral deviation вместо того, чтобы считать каждый reach на верхний ряд простым разгибанием.
- RSNA review по extensor mechanism anatomy: разгибание пальцев — результат взаимодействия intrinsic/extrinsic muscles и retinacular structures, а не независимые приводы для каждого пальца. Это поддерживает coupling и mixed motor-program penalties.
- Материалы Schieber lab по passive/active constraints независимости пальцев: пальцы механически и неврологически связаны. Это аргумент против модели независимых точек.
- Scientific Reports paper про finger dexterity у музыкантов: tapping rate и независимость пальцев зависят от neuromuscular и biomechanical constraints. Это поддерживает timing/recovery модель вместо статического same-finger penalty.
- Работа Vergara et al. по hand anthropometry: размеры кисти важны для ergonomic modelling. Это поддерживает будущие конфигурируемые hand profiles вместо выдумки про универсальную руку.

## Где это отражено в коде

- `finger_axis_deviation`: боковая off-axis коррекция. Мотивация: `ASEF < AWDF`.
- `finger_ready` / `finger_recovery_time`: временная bounce-модель. Мотивация: `FD < FF`, но `ASDFA` уже успевает восстановиться.
- `uniform_motor_program` / `mixed_motor_program_mismatch`: однородность повторяемого блока. Мотивация: `ASDFASDF < AWDFAWDF`.
- `PalmPosture`: грубая модель row-bias/tension кисти. Мотивация: `zxcv` создаёт lower-row posture.
- `cognitive_pattern_adjustment`: слабый salience-бонус для известных клавиатурных форм. Мотивация: люди выбирают паттерны, но очевидные строки не должны доминировать, потому что словари уже их покрывают.
- `load_weights_config`: разреженный слой переопределения для персональной калибровки при захардкоженных defaults.

## Долг по исследованию

Текущая модель всё ещё 2D/2.5D. В будущем можно добавить:

- конфигурируемый hand anthropometry profile;
- матрицу связанности пальцев;
- лучшую Shift posture с занятым мизинцем;
- pairwise tuning по наблюдениям вроде `asef < awdf`;
- опциональное 3D кинематическое приближение, если оно даст измеримый прирост качества ранжирования.

## Direction continuity before a sweep

User testing exposed a ranking error: `fdasdfasdf` appeared too early. The issue is not the literal string; it is the motor-program shape. `FD` is a right-to-left same-row roll, while `ASDF` is a left-to-right four-finger sweep. Entering a sweep immediately after an opposite-direction setup gesture feels less natural than `DF+ASDF`, where the prefix and the sweep point in the same direction.

Implementation notes:

- `pair_direction_continuity_adjustment()` scores adjacent two-key roll direction continuity before the full four-key sweep is even visible. This catches `FDAS` early, before large sweep rewards can hide the problem.
- `pre_sweep_direction_adjustment()` scores the completed form: a two-key same-row prefix immediately followed by a four-key sweep.
- The model remains pattern-general. It does not hardcode `fdasdf`; it compares physical roll direction derived from finger order and key X coordinates.

This is based on the user's motor observation rather than a separate medical paper. It is still consistent with the broader model: smooth repeated motor programs should avoid unnecessary direction changes unless the pattern family intentionally represents a reverse sweep.

## FEW(A/Q) as a coupled reverse upper cluster

User testing found that `fewas` was missing too late. The correction is based on a new observation: `FEWQ` / `FEWA` feels like an FDSA-like reverse roll with the middle and ring fingers slightly extended upward, not like a generic mixed-row pattern.

Implementation notes:

- `upper_reverse_coupled_trigram()` detects the `F->E->W`-like movement: left index home, left middle upper, left ring upper, physical direction right-to-left.
- `upper_reverse_axis_relief()` reduces the isolated ring->W off-axis penalty when W follows E. This does not make `AWDF` cheap, because `AWDF` lacks the E->W preload/coupling context.
- `upper_reverse_split_sweep()` treats `FEWA`/`FEWQ` as a split reverse sweep. It remains slightly harder than `FDSA`, but it is no longer charged as a generic mixed-row failure.
- `home_return_wait_relief()` handles cases such as `FEWAS`: ring W -> pinky A -> ring S is a return toward home/rest, not an arbitrary same-finger bounce.

This is currently user-observation-driven, not sourced from a new medical paper. It is consistent with the already documented finger-coupling and off-axis movement assumptions.

## Implementation note: generation latency and compact beam state

This section is not external anatomy research. It documents an engineering decision that affects reproducibility and future maintenance work.

User testing showed that `gen --min 4 --max 6 --limit very_large | wc -l` produced all expected lines, but the first output was delayed. The old Rust path computed all requested depths, accumulated generated sequences, repeatedly sorted the result set, and only then printed from `main`.

The model itself did not require this delay. The v10 generator therefore separates two concerns:

- `TypingState::push()` keeps full `StepCost` history for `score --explain` and tests.
- `TypingState::push_compact()` keeps only state needed for future scoring and generation. It intentionally drops explanation history.

The generator also pre-resolves charset characters into `Key` values and uses `select_nth_unstable_by` to retain the best beam window before sorting that smaller window. This is a readability-preserving optimization: it avoids turning the model into low-level spaghetti while removing the obvious clone/sort overhead.

The optional `--stream` mode emits each completed depth immediately. This relaxes exact global cross-length ordering, but it is appropriate for pipes into tools such as hashcat where first-line latency and continuous candidate flow matter more than a final globally sorted vector.

## Output deduplication implementation note

The current beam generator usually emits unique strings because the input charset is deduplicated and each string is reached through one path. Therefore output dedupe is intentionally optional and disabled by default.

Two dedupe modes exist:

- `fast`: store a 64-bit FNV-1a fingerprint. In Rust this uses a `HashSet<u64>` with an identity hasher to avoid running a second cryptographic/randomized hash over the fingerprint. This is deliberately not collision-proof. A collision may skip one candidate, which is acceptable for speed-oriented wordlist generation.
- `exact`: store full strings in a `HashSet<String>`. This is lossless but clones/stores every emitted candidate and can cost significant memory on long runs.

This feature is not expected to improve speed in the current single-beam generator. Its purpose is to protect future generation modes where duplicates become realistic: multi-family pattern enumeration, seed-based mutation, custom charset aliases, and route-like generation layers.

## Parallel beam expansion implementation note

This section documents an engineering decision, not a new physiology claim.

The expensive part of generation is beam expansion: for every surviving prefix, the generator evaluates every next key in the charset. These prefix expansions are independent. v12 therefore parallelizes this hot path with Rayon:

- `expand_next_states()` expands the current frontier either sequentially or with `par_iter()` depending on `GenerateOptions::parallel` and `parallel_threshold`.
- The model score is not approximated and the beam is not partitioned into independently ranked shards. All expanded candidates are merged back into one vector, then retained and sorted with the same deterministic comparator as the single-threaded path.
- This means `--engine fast-v1` should preserve the same quality and final order as `--engine reference`, except for any bug-level floating/tie issue. The comparator includes score, average cost, total cost, and text, so equal-score boundary instability should be rare and controllable.
- `--reference` selects the conservative Rust contract engine. `--single-thread` keeps the fast engine on the sequential expansion path for debugging Rayon-specific issues.
- `--jobs N` / `--threads N` caps Rayon worker threads when the user does not want to occupy the whole CPU.
- `--parallel-threshold N` prevents Rayon overhead on tiny first depths where sequential expansion is cheaper.

This is deliberately different from a fast but lower-quality sharded generator. A sharded design could expand independent pattern families or prefix ranges and merge approximately, but that would risk moving worse candidates too early. For now, v12 keeps exact shared beam retention and only parallelizes the embarrassingly parallel scoring work before retention.
