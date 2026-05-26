# Ergokeygen configuration

Ergokeygen does not require a config file. The default model is hardcoded in Rust. A profile file is only a sparse override layer.

The config format is deliberately small and dependency-free:

```ini
[weights]
finger_axis_lateral = 0.95
mixed_motor_program_penalty = 1.00

[finger.left_ring]
lateral_factor = 1.70
axis_dx = 0.32
axis_dy = -1.00
repeat_factor = 1.25
```

Lines starting with `#` or `;` are comments. Section names and keys may use `_` or `-`. Values are floating-point numbers.

## Sections

### `[weights]`

Directly overrides any field from the internal `Weights` structure.

Good first knobs:

- `finger_recovery`: increase when repeated-finger patterns such as `ff`, `dd`, `fdfd` appear too early; decrease if the model over-punishes repeated use after a real pause.
- `timing_wait`: increase when timing stalls should dominate more strongly; decrease if the generator becomes too afraid of any repeated finger.
- `finger_axis_lateral`: increase when off-axis movement such as ring-to-`W` is still too cheap; decrease if upper-row movement becomes unrealistically expensive.
- `mixed_motor_program_penalty`: increase when `AWDF`-like mixed-row blocks appear too early; decrease if useful mixed clusters disappear too aggressively.
- `cognitive_cap_per_step`: increase only if human-obvious patterns are being ignored too much; keep it low for password work because common dictionaries already cover obvious strings.
- `index_stretch`, `half_v_stretch`, `digit_5_stretch`, `digit_6_stretch`: adjust if your keyboard size or hand span makes left-index reach easier or harder than default.
- `left_shift_right_reach`: increase if shifted left-hand typing feels much worse to the right side of the left hand; decrease if you use Shift comfortably or use the opposite Shift.

### `[finger.<name>]`

Shortcut section for per-finger calibration.

Supported names:

- `left_pinky`
- `left_ring`
- `left_middle`
- `left_index`
- `right_index`
- `right_middle`
- `right_ring`
- `right_pinky`

Supported keys:

- `axis_dx`: horizontal component of the relaxed finger extension axis.
- `axis_dy`: vertical component of the relaxed finger extension axis. Usually negative because upper rows have lower Y in the current coordinate system.
- `lateral_factor`: how expensive sideways deviation is for this finger.
- `repeat_factor`: how slowly this finger recovers for repeat/timing purposes.

## How to tune finger axis values

The axis is not a medical skeleton. It is a 2D ergonomic proxy.

Increase `axis_dx` to rotate the finger's easy extension direction to the right. Decrease it to rotate the easy direction left.

Examples:

- If left ring to `W` feels even worse than default, increase `[finger.left_ring].lateral_factor` first. If needed, move `axis_dx` a little to reflect your actual relaxed ring direction.
- If middle to `E` feels very natural, keep `[finger.left_middle].lateral_factor` low and keep the axis close to the `D/E` line.
- If your pinky is short or weak, increase `[finger.left_pinky].repeat_factor` and `[finger.left_pinky].lateral_factor`.
- If you have a large hand span, decrease left-index stretch weights; if laptop spacing or posture makes reaches harder, increase them.

Recommended small-step tuning:

```ini
[finger.left_ring]
lateral_factor = 1.50
```

Then test:

```bash
cargo run -- compare asef awdf --prefer-hand left --mode onehand --config your-profile.ekg
cargo run -- gen --reference --min 4 --max 4 --limit 40 --prefer-hand left --mode onehand --chars lower --config your-profile.ekg --show-score
```

## Why change defaults at all?

Reasons to change defaults:

- your hand is larger/smaller than the assumed relaxed geometry;
- your laptop keyboard has unusual key spacing or stagger;
- your ring/pinky independence differs from the default;
- you type from a shifted wrist angle;
- you want a stricter left-hand-only model;
- you want a weaker or stronger cognitive-pattern component;
- you are generating for a target pattern family rather than general ergonomic candidates.

Reasons not to change defaults:

- you are only using the generator as a broad hashcat candidate source;
- you do not have concrete comparisons such as `asef < awdf` or `fd < ff`;
- you are trying to force obvious strings to the top. Put those in a normal dictionary instead.

## Calibration workflow

Use pairwise comparisons. Do not tune by vibes alone.

Good comparisons:

```text
asdf < fddf
fd < ff
asef < awdf
asdfasdf < awdfawdf
zxcvz < zxcvq
qwer < rewq
1234 < 1256
```

For each change:

```bash
cargo test
cargo run -- compare asef awdf --prefer-hand left --mode onehand --config your-profile.ekg
```

Keep the Rust defaults conservative. Use profile files for personal or experimental tuning.

---

# Конфигурация Ergokeygen

Ergokeygen не требует config-файла. Дефолтная модель захардкожена в Rust. Profile-файл — это только разреженный слой переопределений.

Формат конфига специально маленький и без зависимостей:

```ini
[weights]
finger_axis_lateral = 0.95
mixed_motor_program_penalty = 1.00

[finger.left_ring]
lateral_factor = 1.70
axis_dx = 0.32
axis_dy = -1.00
repeat_factor = 1.25
```

Строки, начинающиеся с `#` или `;`, считаются комментариями. В названиях секций и ключей можно использовать `_` или `-`. Значения — числа с плавающей точкой.

## Секции

### `[weights]`

Напрямую переопределяет любое поле внутренней структуры `Weights`.

Хорошие первые параметры для настройки:

- `finger_recovery`: увеличивай, если паттерны с повтором пальца вроде `ff`, `dd`, `fdfd` появляются слишком рано; уменьшай, если модель слишком сильно штрафует повтор после реальной паузы.
- `timing_wait`: увеличивай, если timing-stall должен сильнее влиять на результат; уменьшай, если генератор слишком боится любого повторного использования пальца.
- `finger_axis_lateral`: увеличивай, если off-axis движение вроде безымянного к `W` всё ещё слишком дешёвое; уменьшай, если верхний ряд стал нереалистично дорогим.
- `mixed_motor_program_penalty`: увеличивай, если блоки вроде `AWDF` появляются слишком рано; уменьшай, если полезные смешанные кластеры исчезают слишком агрессивно.
- `cognitive_cap_per_step`: увеличивай только если очевидные человеческие паттерны слишком игнорируются; для парольной генерации держи низким, потому что общие словари уже покрывают очевидные строки.
- `index_stretch`, `half_v_stretch`, `digit_5_stretch`, `digit_6_stretch`: меняй, если размер клавиатуры или размах руки делает дотягивания левого указательного легче или тяжелее дефолта.
- `left_shift_right_reach`: увеличивай, если shifted-набор левой рукой сильно портится вправо от левого Shift; уменьшай, если Shift используется комфортно или чаще используется противоположный Shift.

### `[finger.<name>]`

Shortcut-секция для настройки конкретного пальца.

Поддерживаемые имена:

- `left_pinky`
- `left_ring`
- `left_middle`
- `left_index`
- `right_index`
- `right_middle`
- `right_ring`
- `right_pinky`

Поддерживаемые ключи:

- `axis_dx`: горизонтальная компонента расслабленной оси разгибания пальца.
- `axis_dy`: вертикальная компонента расслабленной оси разгибания пальца. Обычно отрицательная, потому что верхние ряды имеют меньший Y в текущей системе координат.
- `lateral_factor`: насколько дорого боковое отклонение для этого пальца.
- `repeat_factor`: насколько медленно этот палец восстанавливается для repeat/timing-модели.

## Как настраивать оси пальцев

Ось — это не медицинский скелет. Это 2D ergonomic proxy.

Увеличение `axis_dx` поворачивает лёгкое направление разгибания пальца вправо. Уменьшение поворачивает его влево.

Примеры:

- Если левый безымянный к `W` ощущается ещё хуже дефолта, сначала увеличь `[finger.left_ring].lateral_factor`. Если нужно, немного сдвинь `axis_dx`, чтобы отразить реальное расслабленное направление безымянного.
- Если средний к `E` ощущается очень естественным, держи `[finger.left_middle].lateral_factor` низким и ось близкой к линии `D/E`.
- Если мизинец короткий или слабый, увеличь `[finger.left_pinky].repeat_factor` и `[finger.left_pinky].lateral_factor`.
- Если рука крупная, уменьши stretch-веса левого указательного; если ноутбучная раскладка или поза делают дотягивания тяжелее, увеличь их.

Рекомендуемая настройка маленькими шагами:

```ini
[finger.left_ring]
lateral_factor = 1.50
```

Потом тестируй:

```bash
cargo run -- compare asef awdf --prefer-hand left --mode onehand --config your-profile.ekg
cargo run -- gen --reference --min 4 --max 4 --limit 40 --prefer-hand left --mode onehand --chars lower --config your-profile.ekg --show-score
```

## Зачем вообще менять defaults?

Причины менять defaults:

- твоя рука больше/меньше предполагаемой relaxed geometry;
- у ноутбучной клавиатуры необычный размер клавиш или stagger;
- независимость безымянного/мизинца отличается от дефолта;
- ты печатаешь с другим углом запястья;
- нужен более строгий left-hand-only профиль;
- нужен более слабый или сильный cognitive-pattern компонент;
- генерация идёт под конкретное семейство паттернов, а не под общие эргономичные кандидаты.

Причины не менять defaults:

- генератор нужен просто как широкий источник кандидатов для hashcat;
- нет конкретных сравнений вроде `asef < awdf` или `fd < ff`;
- ты пытаешься насильно поднять очевидные строки в топ. Для них лучше использовать обычный словарь.

## Workflow калибровки

Используй pairwise-сравнения. Не крути веса только по ощущениям.

Хорошие сравнения:

```text
asdf < fddf
fd < ff
asef < awdf
asdfasdf < awdfawdf
zxcvz < zxcvq
qwer < rewq
1234 < 1256
```

После каждого изменения:

```bash
cargo test
cargo run -- compare asef awdf --prefer-hand left --mode onehand --config your-profile.ekg
```

Rust defaults лучше держать консервативными. Для персональной или экспериментальной настройки используй profile-файлы.

### `pre_sweep_direction_change_penalty` and `pre_sweep_direction_match_reward`

These parameters control direction continuity around sweep-like motor programs.

`pre_sweep_direction_change_penalty` raises the cost of patterns where a short same-row roll points one way and the following sweep points the other way. Example: `FD+ASDF` is worse than `DF+ASDF`, because `FD` moves right-to-left and `ASDF` moves left-to-right.

Increase it if reverse-prefix patterns like `fdasdf...` appear too early. Decrease it if the generator becomes too monotone and suppresses too many direction changes.

`pre_sweep_direction_match_reward` is intentionally small. It gives a tiny bonus when the prefix roll and following sweep have the same direction, but it must not dominate the physical model. Strong values here will overproduce smooth but boring same-direction walks.

Recommended ranges:

- `pre_sweep_direction_change_penalty`: `0.70 .. 2.00`
- `pre_sweep_direction_match_reward`: `-0.02 .. -0.20`

Default values:

```ini
pre_sweep_direction_change_penalty = 1.35
pre_sweep_direction_match_reward = -0.12
```

## FEW(A/Q)-style coupled reverse clusters

These weights exist because `FEWQ` / `FEWA` can be a natural FDSA-like reverse roll: `F` with index, `E` with middle, `W` with ring, then `Q` or `A` with pinky. The key point is context. `W` after `E` can be easier than isolated ring-to-W movement because middle-finger extension mechanically biases the neighbouring ring finger toward W.

Config keys:

```ini
upper_reverse_coupled_roll_reward = -0.36
upper_reverse_axis_relief = 0.95
upper_reverse_split_sweep_reward = -1.15
home_return_wait_relief = 0.92
```

Tuning:

- Increase `upper_reverse_axis_relief` if `FEWA`/`FEWQ` are still too rare compared to `FDSA` and `ASEF`.
- Decrease `upper_reverse_axis_relief` if `AWDF`-like patterns become too cheap. The relief must stay context-sensitive.
- Make `upper_reverse_split_sweep_reward` more negative if `FEWA`/`FEWQ` should rank closer to `FDSA`.
- Make it less negative if split-row reverse clusters start dominating clean home-row sweeps.
- Increase `home_return_wait_relief` if `FEWAS` is over-penalized by ring W -> A -> S timing. Decrease it if same-finger returns become suspiciously cheap.
