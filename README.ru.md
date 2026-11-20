# AEConv — Конвертер любых расширений

[![Crates.io](https://img.shields.io/crates/v/aeconv.svg)](https://crates.io/crates/aeconv)
[![Latest Release](https://img.shields.io/github/v/release/NirkZxc/aeconv)](https://github.com/NirkZxc/aeconv/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Кроссплатформенная консольная утилита для конвертации изображений между различными форматами. Написана на Rust для производительности и надёжности.

## Возможности

- **Широкая поддержка форматов**: PNG, JPEG, WebP, BMP, TIFF, TGA, GIF, QOI, PNM (PPM/PGM/PBM), ICO, ICNS (только кодирование), SVG
- **Кроссплатформенность**: Windows, Linux, macOS
- **Высокая производительность**: Построена на `image` crate и `resvg` для растеризации SVG
- **Многоразрешающие иконки**: ICO (декод/кодек) и ICNS (только кодирование) с несколькими стандартными размерами
- **Растеризация SVG**: Конвертация SVG в любой растровый формат
- **Контроль качества**: Настраиваемое качество для форматов с потерями (JPEG, WebP)
- **Атомарные записи**: Безопасные файловые операции с атомарным переименованием
- **Поддержка Unicode**: Полная поддержка имён файлов и путей с Unicode и пробелами
- **Изменение размера**: Изменение размера изображения до точных размеров с высококачественным фильтром Lanczos3 (`--resize WxH`)
- **Скругление углов**: Скругление углов изображения с радиусом в пикселях или процентах (`--round-corners Npx` / `N%`)

## Установка

### Из Cargo (рекомендуется)

```bash
cargo install aeconv
```

### Готовые бинарные файлы

Скачайте последний релиз для вашей платформы на [странице релизов](https://github.com/NirkZxc/aeconv/releases).

### Из исходного кода

```bash
git clone https://github.com/NirkZxc/aeconv
cd aeconv
cargo build --release
# Бинарный файл будет в target/release/aeconv
```

## Использование

### Базовые конвертации

```bash
# Конвертация с помощью предопределённых команд
aeconv jpg-to-ico photo.jpg
aeconv png-to-webp image.png
aeconv jpg-to-png photo.jpg

# Общая конвертация с явным указанием форматов
aeconv convert --from png --to jpg image.png
aeconv convert --from jpg --to ico photo.jpg

# С указанием пути для результата
aeconv jpg-to-ico photo.jpg -o /path/to/output.ico

# Расширение выходного файла должно соответствовать целевому формату (валидация)
aeconv convert --from png --to webp image.png --output result.webp   # OK
aeconv convert --from png --to webp image.png --output result.png    # Ошибка: расширение не совпадает

# С контролем качества (1-100)
aeconv convert --from png --to jpg image.png -q 85

# Фон для сведения прозрачности при конвертации в JPEG
aeconv convert --from png --to jpg image.png --background "#222222"
aeconv convert --from png --to jpg image.png -b "#fff"
aeconv convert --from png --to jpg image.png --background "255,0,128"

# Принудительная перезапись существующих файлов
aeconv png-to-webp image.png --force
```

### Информационные команды

```bash
# Список всех доступных операций конвертации
aeconv list

# Список всех поддерживаемых форматов
aeconv formats

# Информация об изображении
aeconv info photo.jpg

# Показать версию
aeconv --version
```

### Поддерживаемые форматы

| Формат | Расширения | Декод | Кодек | Прозрачность | Анимация |
|--------|-----------|-------|-------|--------------|----------|
| PNG | .png | ✅ | ✅ | ✅ | ❌ |
| JPEG | .jpg, .jpeg, .jpe, .jfif | ✅ | ✅ | ❌ | ❌ |
| WebP | .webp | ✅ | ✅ | ✅ | ✅ |
| BMP | .bmp, .dib | ✅ | ✅ | ❌ | ❌ |
| TIFF | .tiff, .tif | ✅ | ✅ | ✅ | ✅ |
| TGA | .tga | ✅ | ✅ | ✅ | ❌ |
| GIF | .gif | ✅ | ✅ | ✅ | ✅ |
| QOI | .qoi | ✅ | ✅ | ✅ | ❌ |
| PNM | .ppm, .pgm, .pbm, .pnm | ✅ | ✅ | ❌ | ❌ |
| ICO | .ico | ✅ | ✅ | ✅ | ❌ |
| ICNS | .icns | ❌ | ✅ | ✅ | ❌ |
| SVG | .svg, .svgz | ✅ | ❌ | ✅ | ❌ |

**Примечание**: Кодирование SVG (векторизация) не поддерживается в Phase 1. Для трассировки растра в вектор используйте специализированные инструменты вроде `potrace`.

**Примечание о прозрачности**: GIF поддерживает бинарную прозрачность (прозрачные или непрозрачные элементы палитры), но не полноценный 8-битный альфа-канал.

**Примечание об анимированных форматах**: WebP, TIFF и GIF отмечены как поддерживающие анимацию в таблице форматов, так как они *могут* содержать несколько кадров. Однако AEConv Phase 1 обрабатывает только первый кадр при декодировании этих форматов. Поддержка многофреймовых форматов будет добавлена в Phase 2.

## Опции

| Опция | Краткая | Описание |
|-------|---------|----------|
| `--output` | `-o` | Путь выходного файла (по умолчанию: та же директория что и входной, с новым расширением) |
| `--force` | `-f` | Перезаписать существующий выходной файл |
| `--quality` | `-q` | Качество для форматов с потерями (1-100, по умолчанию: 90 для JPEG, 80 для WebP) |
| `--background` | `-b` | Цвет фона при сведении прозрачности (`#RGB`, `#RRGGBB` или `R,G,B`; по умолчанию: белый) |
| `--resize` | | Изменить размер до точных WIDTHxHEIGHT пикселей с фильтром Lanczos3 (например, `--resize 512x512`) |
| `--round-corners` | | Скруглить углы: радиус в пикселях (`32px`) или процент от меньшей стороны (`20%`) |
| `--verbose` | `-v` | Показать подробную информацию о конвертации |
| `--help` | `-h` | Показать справку |
| `--version` | `-V` | Показать версию |

## Обработка изображений

### Изменение размера (`--resize`)

Изменение размера изображения до точных размеров с высококачественным фильтром Lanczos3:

```bash
# Уменьшить изображение
aeconv input.png output.png --resize 512x512

# Увеличить изображение
aeconv input.png output.png --resize 2000x1500

# Неквадратный размер (соотношение сторон может измениться)
aeconv input.png output.png --resize 800x600

# Изменение размера вместе с конвертацией формата
aeconv input.png output.webp --resize 512x512
```

Размеры должны быть больше нуля. Запрос на избыточный размер отклоняется для предотвращения чрезмерного выделения памяти.

### Скругление углов (`--round-corners`)

Скругление углов изображения. Радиус указывается в пикселях или процентах от меньшей стороны:

```bash
# Фиксированный радиус в пикселях
aeconv input.png output.png --round-corners 32px

# Радиус в процентах от меньшей стороны
aeconv input.png output.png --round-corners 20%

# Радиус автоматически ограничивается, чтобы углы не накладывались
aeconv input.png output.png --round-corners 50%
```

Угла становятся прозрачными с плавными анти-алиасированными краями; разрешение изображения не меняется. Для форматов без прозрачности (например, JPEG) углы сглаживаются на цвет фона `--background`.

### Совместная обработка

Изменение размера выполняется до скругления углов, поэтому обе операции работают вместе без промежуточных файлов:

```bash
# Изменить размер и скруглить углы, конвертируя в WebP
aeconv input.png output.webp --resize 512x512 --round-corners 15%

# Изменить размер, скруглить углы и задать цвет фона для форматов без альфа-канала
aeconv input.png output.jpg --resize 400x400 --round-corners 10% --background "#222222"
```

## Примеры

```bash
# Конвертация PNG в WebP с высоким качеством
aeconv png-to-webp image.png -q 95

# Пакетная конвертация всех JPEG в WebP (через shell)
for f in *.jpg; do aeconv jpg-to-webp "$f"; done

# Конвертация SVG в PNG для веба
aeconv convert --from svg --to png logo.svg

# Создание многоразрешающего ICO из PNG
aeconv png-to-ico icon.png

# Конвертация в ICNS для macOS
aeconv convert --from png --to icns icon.png

# Получить детали изображения
aeconv info photo.jpg
```

## Архитектура

AEConv построена с модульной архитектурой:

- **cli** — Парсинг аргументов командной строки через `clap`
- **engine** — Основной конвейер конвертации
- **formats** — Реестр форматов с возможностями
- **codecs** — Декодирование/кодирование для каждого формата
- **metadata** — Работа с метаданными изображений
- **output** — Определение пути выходного файла и атомарная запись
- **errors** — Централизованная обработка ошибок с понятными сообщениями

## Сборка

### Требования

- Rust 1.70+ (stable)
- Cargo

### Команды сборки

```bash
# Debug сборка
cargo build

# Release сборка (оптимизированная)
cargo build --release

# Запуск тестов
cargo test

# Кросс-компиляция для Windows из Linux
cargo build --release --target x86_64-pc-windows-gnu

# Кросс-компиляция для Linux из Windows
cargo build --release --target x86_64-unknown-linux-gnu
```

## Лицензия

MIT License — подробности в [LICENSE](LICENSE).

## Участие в разработке

См. [CONTRIBUTING.md](CONTRIBUTING.md) для руководства.

## История изменений

См. [CHANGELOG.md](CHANGELOG.md) для истории версий.
