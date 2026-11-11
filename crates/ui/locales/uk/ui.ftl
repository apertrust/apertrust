# Інтерфейс Apertrust українською (docs/gui.md, Translation).
# Переклад повідомлень з en-GB/ui.ftl з тими самими змінними. Чернетка: ще не перевірена
# носієм мови.

app-name = Apertrust

## Загальні дії на верхній панелі

action-import = Імпорт…
action-export = Експорт…
action-export-again = Експортувати ще раз
action-reset = Скинути
action-undo = Скасувати
action-redo = Повторити
action-copy-edit = Копіювати обробку
action-apply-edit = Застосувати обробку
action-copy-apply-parts = Копіювати або застосувати частину
action-settings = Налаштування
status-saved = Збережено

## Вкладки бічної панелі

tab-photos = Фото
tab-base = Базові
tab-color = Колір
tab-correct = Корекція
tab-effect = Ефект
tab-history = Історія
tab-metadata = Метадані
panel-tab-later = { $tab }: з'явиться в одній із наступних версій
panel-tabs = Вкладки бічної панелі
panel-more-tabs = Інші вкладки
panel-tab-options = Параметри вкладок
panel-show-tab-names = Показувати назви вкладок
panel-customise-tabs = Налаштувати вкладки…
panel-width = Ширина бічної панелі

## Кнопки в заголовку розділу; $section — назва розділу

section-reset = Скинути: { $section }
section-presets = Пресети: { $section }
section-options = Параметри: { $section }
section-toggle = Відкрити або закрити: { $section }. Утримуйте H над розділом, щоб побачити знімок без нього.
section-tabs = Вкладки: { $section }
section-tab-later = З'явиться в одній із наступних версій
choice-option-later = З'явиться в одній із наступних версій
choice-option-unavailable = Недоступно для цього знімка

## Згортана група в модулі, згортаний розділ darktable

collapsible-group-set = Змінено: { $count }
collapsible-group-tip = { $group }: змінено { $count }

## Рядок пресетів у модулі, як Режим балансу білого

preset-custom = Власні
preset-option-unavailable = Недоступно для цього знімка

## Позначка повзунка

slider-kept = Зафіксовано: ви встановили це значення, тож Авто та Зіставлення його не змінюють. Клацніть, щоб звільнити.
slider-keep = Клацніть, щоб зафіксувати це значення.
marker-keep = Зафіксувати: { $control }
field-value = { $control }: точне значення
field-value-in = { $control }, в { $unit }
slider-before-match = До Зіставлення: { $value }
slider-value-tip = Клацніть або натисніть Enter на повзунку, щоб ввести значення.
slider-value-tip-mac = Клацніть або натисніть Return на повзунку, щоб ввести значення.

## Теми

theme-dark = Темна
theme-light = Світла
theme-high-contrast = Висока контрастність

## Кількість

photo-count =
    { $count ->
        [one] { $count } знімок
        [few] { $count } знімки
        [many] { $count } знімків
       *[other] { $count } знімка
    }

## Одиниці, після значення

unit-ev = EV
unit-percent = %
unit-degrees = °
unit-kelvin = K
unit-pixels = px
unit-iso = ISO
unit-mib = МіБ
unit-gb = ГБ

## Значення з одиницею, як його показує повзунок і чують екранні читачі; $value уже у
## форматі чисел мови; одиниці як у uk.po darktable

unit-ev-value = { $value } EV
unit-percent-value = { $value }%
unit-degrees-value = { $value }°
unit-kelvin-value = { $value } K
unit-pixels-value = { $value } px
unit-iso-value = { $value } ISO

## Верхня панель

top-panel = Верхня панель
top-panel-menu = Меню
top-panel-more = Інші дії
status-saving = Збереження…
status-saving-tip = Обробку цього фото ще записують поруч із ним
status-not-saved = Не збережено
status-not-saved-reason = Не збережено: { $error }
status-view-only = Лише перегляд
window-minimise = Згорнути
window-maximise = Розгорнути
window-restore = Відновити
window-close = Закрити

## Завдання: індикатор у кінці верхньої панелі і вікно, яке він відкриває

jobs-name = Завдання
action-show-jobs = Завдання
jobs-tip = Завдання: { $value }
jobs-value-running = Виконується: { $running }
jobs-value-waiting = Очікують: { $waiting }
jobs-value-running-waiting = Виконується: { $running }, очікують: { $waiting }
jobs-value-failed = Помилки: { $failed }
jobs-empty = Завдань немає.
jobs-queued = Очікує на інші завдання
jobs-finished = Завершено: { $message }
jobs-failed = Не вдалося: { $message }
jobs-cancelled = Скасовано: { $message }
jobs-cancel = Скасувати
jobs-cancel-tip = Зупинити це завдання
jobs-clear-finished = Прибрати завершені
jobs-live-started = Почато: { $message }

## Повідомлення самих завдань

job-import = Імпортування { $done } / { $total }
job-delete = Видалення { $done } / { $total }
job-thumbnails = Створення мініатюр
job-folder = Читання { $folder }

## Очищення кешу: $kind — що очищається, thumbnails чи pipelines; $done — видалено записів
## із $total; $entries — скільки видалено, $size і $unit — скільки вони займали, одиниця
## kb, mb чи gb.

job-cache-size =
    { $size } { $unit ->
        [kb] кБ
        [gb] ГБ
       *[mb] МБ
    }
job-cache-entries =
    { $kind ->
        [thumbnails]
            { $entries ->
                [one] { $entries } мініатюру
                [few] { $entries } мініатюри
                [many] { $entries } мініатюр
               *[other] { $entries } мініатюри
            }
       *[pipelines]
            { $entries ->
                [one] { $entries } конвеєр
                [few] { $entries } конвеєри
                [many] { $entries } конвеєрів
               *[other] { $entries } конвеєра
            }
    }
job-cache-clear =
    { $kind ->
        [thumbnails] Очищення кешу мініатюр
       *[pipelines] Очищення кешу конвеєрів
    }
job-cache-clear-now = Видалено { $done } із { $total }
job-cache-cleared = Видалено { job-cache-entries }, { job-cache-size }
job-cache-clear-cancelled = Очищення скасовано: видалено { job-cache-entries }, { job-cache-size }

job-export-photo =
    Експортування { $number } / { $total } до { $storage ->
       *[disk] файлу на диску
    }
job-export-queued = Далі в черзі: { photo-count }, до папки { $folder }
job-export-now =
    { $total ->
        [1] Експортується { $photo }
       *[other] Готово { $done } із { $total } · зараз { $photo }
    }
job-export-between = Готово { $done } із { $total }
job-export-finished =
    { $skipped ->
        [0]
            { $written ->
                [1] Знімок експортовано
               *[other] Експортовано знімків: { $written }
            }
       *[other]
            { $total ->
                [1] Знімок пропущено
               *[other] Експортовано { $written } із { $total }, пропущено: { $skipped }
            }
    }
job-export-failed-outcome =
    { $total ->
        [1] Не вдалося експортувати знімок
       *[other] Експорт не вдався. Експортовано { $written } із { $total }
    }
job-export-cancelled =
    { $total ->
        [1] Експорт скасовано
       *[other] Експорт скасовано. Експортовано { $written } із { $total }
    }

job-cause-unreadable = не вдалося прочитати файл raw
job-cause-edit = не вдалося прочитати його редагування
job-cause-binding = змінений модуль не вдалося застосувати до фото
job-cause-size = заданий розмір не підходить для цього формату
job-cause-pattern = ім'я файлу недійсне
job-cause-name = ім'я файлу не дає цьому фото назви
job-cause-render = не вдалося обробити фото
job-cause-encode = не вдалося закодувати файл
job-cause-disk-full = диск заповнений
job-cause-read-only = диск або папка лише для читання
job-cause-gone = папки або її диска більше немає
job-cause-folder = папку не вдається створити або записати до неї
job-cause-write = не вдалося записати файл
job-cause-engine = рушій несподівано зупинився
job-cause-cache-remove = деякі файли кешу не вдалося видалити
job-cause-cache-read = не вдалося прочитати теку кешу

## Рядок завдання у вікні завдань: рядки і кнопки

activity-folder = Папка: { $folder }
activity-folder-at = Папка: { $folder } · { $time }
activity-failure = { $photo }: не вдалося записати { $file }: { $cause }.
activity-failure-photo = { $photo }: { $cause }.
activity-failure-cause = { $cause }.
activity-show-finder = Показати у Finder
activity-show-explorer = Показати у Провіднику
activity-show-files = Показати у Файлах
activity-show-tip = Відкрити папку експорту
activity-try-again = Повторити
activity-try-again-tip = Експортувати знімки, які не вдалося експортувати, ще раз з тими самими параметрами
activity-dismiss = Закрити

## Центральна смуга, над знімком

central-bar = Центральна смуга
photo-show-selected = Показати всі вибрані знімки
photo-proof-margin = Поле перевірки
photo-before-after = До і після
photo-guides = Напрямні
guides-show-label = Показати напрямні
guides-show-full = Показати напрямні
guides-show-promise = Показати накладені напрямні лінії, коли цей модуль має фокус.
guides-settings-full = Змінити глобальні налаштування напрямних ліній
guides-settings-promise = Ці налаштування застосовуються глобально і впливатимуть на будь-який модуль, який показує накладення напрямних.
photo-layer = Шар: { $layer }. Змінити шар
photo-new-layer = Новий шар
photo-values = Значення під курсором
photo-values-read = Червоний { $red }, зелений { $green }, синій { $blue }, сірий { $grey }
photo-zoom = Масштаб
photo-zoom-small = Малий
photo-zoom-fit = За розміром
photo-zoom-fill = Заповнити
photo-zoom-in = Збільшити
photo-zoom-out = Зменшити
photo-zoom-tip = Масштаб: зведення пальців або { $command }+прокручування; Z — 100%. Прокручування рухає збільшений знімок, Shift+прокручування — убік
photo-zoom-tip-exact = На 100% екран показує саме те, яким буде експорт. Масштаб: зведення пальців або { $command }+прокручування; Z — 100%. Прокручування рухає збільшений знімок, Shift+прокручування — убік
photo-zoom-later = Масштаб з'явиться разом із переглядом 100%
layer-image = Знімок

## Центральна панель

photo-opening = Відкриваю { $name }…
photo-rendering = Рендеринг…
controls-opening = Відкриваю { $name }…
edit-not-saved = Обробку { $name } не збережено: { $error }
photo-view-only = Оброблено в новішому Apertrust, { $version }: відкрито лише для перегляду
photo-view-only-unknown = Оброблено в новішому Apertrust: відкрито лише для перегляду
photo-shutter = { $value } с
photo-aperture = f/{ $value }
photo-focal-length = { $value } мм
photo-iso = ISO { $value }

## Діалог відкриття, з меню Файл › Відкрити…

open-title = Відкрити
open-filter = Raw-знімки

## Панель сітки та її комірки

grid-panel = Панель сітки
grid-cell-edited = Оброблено
grid-cell-view-only = Лише перегляд: редагування записала новіша версія Apertrust
grid-cell-unreadable = Не вдалося прочитати редагування: { $reason }
grid-cell-rating =
    { $count ->
        [0] Без зірок
        [one] { $count } зірка
        [few] { $count } зірки
        [many] { $count } зірок
       *[other] { $count } зірки
    }
grid-cell-rejected = Відхилено
grid-cell-labels = Кольорові мітки: { $labels }
grid-cell-rate =
    { $count ->
        [one] Оцінити в { $count } зірку
        [few] Оцінити в { $count } зірки
        [many] Оцінити в { $count } зірок
       *[other] Оцінити в { $count } зірки
    }
grid-cell-reject = Відхилити
grid-cell-colour-labels = Кольорові мітки
grid-cell-no-labels = Немає
grid-cell-clear-labels = Зняти мітки
colour-label-red = Червоний
colour-label-yellow = Жовтий
colour-label-green = Зелений
colour-label-blue = Синій
colour-label-purple = Фіолетовий
grid-panel-height = Висота панелі сітки
grid-rows =
    { $count ->
        [one] { $count } рядок
        [few] { $count } рядки
        [many] { $count } рядків
       *[other] { $count } рядка
    }
grid-panel-width = Ширина панелі сітки
grid-columns =
    { $count ->
        [one] { $count } стовпець
        [few] { $count } стовпці
        [many] { $count } стовпців
       *[other] { $count } стовпця
    }
grid-bar-options = Параметри панелі сітки
grid-position = Розташування
grid-bar = Смуга сітки
grid-sort = Сортування
grid-sort-value =
    { $key ->
        [capture-time] Час зйомки
        [file-name] Назва файлу
        [edited] Оброблені
       *[rating] Рейтинг
    }, { $descending ->
        [yes] за спаданням
       *[no] за зростанням
    }
grid-sort-tip =
    Сортування: { $key ->
        [capture-time] час зйомки
        [file-name] назва файлу
        [edited] оброблені
       *[rating] рейтинг
    }, { $descending ->
        [yes] за спаданням
       *[no] за зростанням
    }
grid-thumbnail-size = Розмір мініатюр
grid-thumbnail-size-value =
    { $size ->
        [small] Малі
        [medium] Середні
        [large] Великі
       *[extra-large] Дуже великі
    }
grid-thumbnail-size-tip =
    Розмір мініатюр: { $size ->
        [small] малі
        [medium] середні
        [large] великі
       *[extra-large] дуже великі
    }
grid-labels = Підписи

## Що відкрито, за видом

open-kind-folder = Тека: { $name }

## Режим теки: Файл › Відкрити теку…, розділ «Теки» вкладки «Бібліотека» та порожні стани

action-open-folder = Відкрити теку…
open-folder-title = Відкрити теку
section-browse = Огляд
browse-folders = Теки
browse-open-name = Що відкрито
browse-no-folder = Жодної теки не відкрито
browse-recents-none = Немає нещодавніх тек
browse-recent-missing = { $label }, не знайдено
browse-try-again = Спробувати ще раз
browse-locate = Знайти…
browse-remove = Прибрати зі списку
browse-locate-title = Знайти теку
browse-plus = Створити або відкрити
action-new-library = Нова бібліотека…
action-new-project = Новий проєкт…
browse-folder-menu = Меню теки { $name }
browse-open-row = Відкрити
browse-open-this-folder = Відкрити цю теку
browse-reading = Читання…
browse-unreadable = Не вдалося прочитати. Спробувати ще раз
folders-reading = Читання { $name }…
folders-recent-missing = Цю теку не вдалося відкрити. { $reason }
folders-recent-gone = Цієї теки там більше немає.
folder-reading = Читання { $folder }…
folder-empty = Немає доступних фото
folder-empty-hint = Виберіть підтеку в розділі «Огляд».
folder-failed = Не вдалося відкрити { $folder }: { $reason }
folder-cancelled = Читання { $folder } скасовано
folder-no-photo = Знімок не вибрано

## Вікно запуску, яке показується замість головного вікна, коли нічого не відкрито

start-recents = Нещодавні
start-recents-none = Ще нічого не відкрито
start-kind-folder = Тека
start-recent-gone = Не знайдено
start-recent-failed = Не вдалося відкрити
start-recent-about = { $kind }, { $path }
start-recent-about-missing = { $kind }, { $path }. { $why }
start-browse = Огляд…
start-open = Відкрити
start-copyright = © 2026
start-tagline =
    Вільний, недеструктивний
    редактор raw-знімків

## Налаштування

settings-close = Закрити
settings-overridden = Задано через { $flag } для цього запуску
settings-after-restart = Набуде чинності після перезапуску
settings-changed = Змінено порівняно з типовим значенням
settings-reset = Скинути
settings-page-general = Загальні
settings-page-import = Імпорт
settings-page-photos = Фото
settings-page-editing = Редагування
settings-page-processing = Обробка
settings-page-storage = Зберігання
settings-page-shortcuts = Гарячі клавіші
settings-page-presets = Шаблони
settings-page-ai = ШІ
settings-page-later = З'явиться в одній із наступних версій
settings-group-interface = Інтерфейс
settings-group-start = Запуск
settings-group-confirmations = Підтвердження
settings-group-thumbnails = Мініатюри
settings-group-side-panel = Бічна панель
settings-group-gpu = GPU
settings-group-diagnostics = Діагностика
settings-group-cache = Кеш
setting-theme-label = Тема
setting-theme-option-dark = Темна
setting-theme-option-light = Світла
setting-theme-option-high-contrast = Висока контрастність
setting-locale-label = Мова
setting-scale-label = Розмір інтерфейсу
setting-sliders-marker-label = Маркер повзунка
setting-sliders-marker-option-triangle = Трикутник
setting-sliders-marker-option-circle = Коло
setting-sliders-marker-option-square = Квадрат
setting-library-reopen-last-label = Відкривати те, що було відкрито востаннє
setting-library-reopen-last-help = Відкриває теку, з якою ви працювали, коли { app-name } закрився, разом з її фото. Файл або тека, відкриті з файлового менеджера чи термінала, відкриваються замість неї.
setting-export-ask-before-overwrite-label = Питати перед перезаписом експортованого файлу
setting-grid-panel-square-thumbnails-label = Квадратні мініатюри
setting-library-raw-thumbnails-from-label = Мініатюри з raw від розміру
setting-library-raw-thumbnails-from-help = Для знімків без змін: мініатюри, більші за цей розмір, рендеряться з raw-файлу в { app-name }, а не беруться з вбудованого JPEG камери. Повільніше. Знімки зі змінами завжди показують свої зміни.
setting-library-raw-thumbnails-from-option-always = Завжди
setting-library-raw-thumbnails-from-option-small = Малий
setting-library-raw-thumbnails-from-option-VGA = VGA
setting-library-raw-thumbnails-from-option-720p = 720p
setting-library-raw-thumbnails-from-option-1080p = 1080p
setting-library-raw-thumbnails-from-option-WQXGA = WQXGA
setting-library-raw-thumbnails-from-option-4K = 4K
setting-library-raw-thumbnails-from-option-5K = 5K
setting-library-raw-thumbnails-from-option-6K = 6K
setting-library-raw-thumbnails-from-option-8K = 8K
setting-library-raw-thumbnails-from-option-never = Ніколи
setting-library-raw-thumbnails-from-option-auto = Авто
setting-side-panel-tab-names-label = Показувати назви вкладок
setting-engine-adapter-label = Відеокарта
setting-engine-adapter-hint = Автоматично
setting-engine-memory-budget-label = Бюджет пам'яті
setting-cache-limited-label = Обмежити розмір кешу
setting-cache-limited-help = Якщо вимкнено, кеш не обмежений: з нього нічого не видаляється, щоб утримати його в межах розміру.
setting-cache-limit-label = Обмеження кешу
setting-cache-limit-help = Понад цей розмір видаляються мініатюри, якими давно не користувалися; за потреби їх буде створено знову.
setting-cache-unused-after-label = Кеші бібліотек без використання
setting-cache-unused-after-help = Кеш бібліотеки, який на цьому комп'ютері не відкривали стільки часу, буде запропоновано видалити. Без вас нічого не видаляється.
setting-cache-unused-after-option-days_30 = 30 днів
setting-cache-unused-after-option-days_90 = 90 днів
setting-cache-unused-after-option-days_180 = 180 днів
setting-cache-unused-after-option-days_365 = 365 днів
setting-cache-unused-after-option-never = Ніколи
setting-log-keep-label = Вести журнал цього сеансу
setting-log-keep-help = Зберігає журнал цього сеансу в пам'яті з моменту ввімкнення, для Довідка › Показати журнал. Щоб вести журнал від самого початку, перезапустіть Apertrust. У файл нічого не записується, доки ви його не збережете.
setting-log-domains-label = Журнал
setting-log-domains-help = Що журнал зберігає, крім попереджень і помилок, які він зберігає завжди. «Докладно» додає набагато більше, для розробника.
setting-log-domains-option-gpu = GPU
setting-log-domains-option-raw = Raw
setting-log-domains-option-pipe = Конвеєр
setting-log-domains-option-perf = Продуктивність
setting-log-domains-option-memory = Пам'ять
setting-log-domains-option-settings = Налаштування
setting-log-domains-option-control = Керування
setting-log-domains-option-event = Події
setting-log-domains-option-verbose = Докладно

## Налаштування › Сховище, його частина, зроблена вручну (рішення 76). $flag і $variable
## — що вибрало теку; $kind — thumbnails, pipelines чи libraries; $entries, $size і $unit —
## як в очищенні кешу; $limit — обмеження в ГБ.

settings-group-folders = Теки
settings-group-cache-contents = У кеші
settings-storage-user-data = Тека даних користувача
settings-storage-cache = Тека кешу
settings-storage-chosen-flag = Задано через { $flag }
settings-storage-chosen-variable = Задано через { $variable }
settings-storage-chosen-user-data = Усередині теки даних користувача
settings-storage-chosen-platform = Системна тека
settings-storage-none = Немає: система не дає теки
settings-storage-show-tip = Відкрити цю теку
settings-storage-show-name = { $show }: { $folder }
settings-storage-kind-thumbnails = Мініатюри
settings-storage-kind-pipelines = Конвеєри
settings-storage-kind-libraries = Кеші бібліотек
settings-storage-entries =
    { $kind ->
        [thumbnails]
            { $entries ->
                [one] { $entries } мініатюра
                [few] { $entries } мініатюри
                [many] { $entries } мініатюр
               *[other] { $entries } мініатюри
            }
        [libraries]
            { $entries ->
                [one] { $entries } бібліотека
                [few] { $entries } бібліотеки
                [many] { $entries } бібліотек
               *[other] { $entries } бібліотеки
            }
       *[pipelines]
            { $entries ->
                [one] { $entries } конвеєр
                [few] { $entries } конвеєри
                [many] { $entries } конвеєрів
               *[other] { $entries } конвеєра
            }
    }
settings-storage-usage = { settings-storage-entries }, { job-cache-size }
settings-storage-total = Разом
settings-storage-total-limit = { job-cache-size } із { $limit } ГБ
settings-storage-total-unlimited = { job-cache-size }, без обмеження
settings-storage-counting = Підрахунок…
settings-storage-clear = Очистити
settings-storage-clear-thumbnails = Очистити мініатюри
settings-storage-clear-pipelines = Очистити конвеєри
settings-storage-clear-tip = Видаляються з диска й створюються знову за потреби
settings-storage-no-cache = Дискового кешу немає: його теку не вдалося записати під час запуску Apertrust, тож між запусками нічого не зберігається.
settings-storage-refused-clearing = Їх уже очищують.
settings-storage-refused-no-cache = Немає дискового кешу, який можна очистити.
settings-storage-refused = Їх не вдалося очистити.

## Вікно журналу, Довідка › Показати журнал; його фільтри — варіанти рядка «Журнал»

action-show-log = Показати журнал
log-name = Журнал
log-off-title = Журнал не ведеться
log-off-text = Увімкніть його в налаштуваннях, і відтоді рядки зберігатимуться в пам'яті, лише для цього сеансу.
log-open-settings = Відкрити налаштування діагностики…
log-filters = Показати
log-filter-warnings = Попередження і помилки
log-filter-not-kept = Не зберігається: увімкніть у Налаштування › Загальні › Діагностика
log-lines = Рядки
log-dropped = Найстаріших рядків уже немає: { $count }. Журнал зберігає 20 000 найновіших.
log-newer = ↓ Новіші рядки: { $count }
log-counts = Збережено рядків: { $kept } · показано: { $shown } · лише цей сеанс, у пам'яті
log-names-files = Рядки називають ваші файли й теки.
log-save = Зберегти у файл…
log-copy = Копіювати в буфер обміну
log-save-title = Зберегти журнал

## Про Apertrust

action-about = Про { app-name }
about-version = Версія { $version }
about-copyright = Авторське право © 2026 розробники { app-name }.
about-free-software = Вільне програмне забезпечення за ліцензією GNU General Public License версії 3 або пізнішої.
about-credits = Подяки
about-licence = Ліцензія
about-back = Назад

## Дії, як їх називає карта клавіш

action-open = Відкрити…
action-view-edit = Перейти до обробки
action-show-browse = Огляд
action-panel-tab-1 = Вкладка бічної панелі 1
action-panel-tab-2 = Вкладка бічної панелі 2
action-panel-tab-3 = Вкладка бічної панелі 3
action-panel-tab-4 = Вкладка бічної панелі 4
action-panel-tab-5 = Вкладка бічної панелі 5
action-scale-up = Збільшити інтерфейс
action-scale-down = Зменшити інтерфейс
action-scale-reset = Звичайний розмір інтерфейсу
action-next-region = Наступна панель
action-previous-region = Попередня панель
action-full-screen = На весь екран
action-close-window = Закрити вікно
action-close = Закрити
action-open-recent-1 = Відкрити недавнє 1
action-open-recent-2 = Відкрити недавнє 2
action-open-recent-3 = Відкрити недавнє 3
action-open-recent-4 = Відкрити недавнє 4
action-open-recent-5 = Відкрити недавнє 5
action-open-recent-6 = Відкрити недавнє 6
action-open-recent-7 = Відкрити недавнє 7
action-open-recent-8 = Відкрити недавнє 8
action-open-recent-9 = Відкрити недавнє 9
action-open-recent-10 = Відкрити недавнє 10
action-clear-recent = Очистити меню
action-quit = Вийти
action-peek = Показати знімок без розділу під вказівником
action-next-photo = Наступний знімок
action-previous-photo = Попередній знімок
action-zoom-in = Збільшити
action-zoom-out = Зменшити
action-zoom-fit = За розміром
action-zoom-100 = Масштаб 100%
action-sort-capture-time = Час зйомки
action-sort-file-name = Назва файлу
action-sort-edited = Оброблені
action-sort-rating = Рейтинг
action-sort-ascending = За зростанням
action-sort-descending = За спаданням
action-thumbnails-small = Малі
action-thumbnails-medium = Середні
action-thumbnails-large = Великі
action-thumbnails-extra-large = Дуже великі
action-grid-position-left = Ліворуч
action-grid-position-right = Праворуч
action-grid-position-bottom = Унизу
action-show-labels = Показувати підписи
action-background-black = Чорне
action-background-very-dark = Дуже темне
action-background-dark = Темне
action-background-medium = Середнє
action-background-light = Світле
action-background-white = Біле
key-space = Пробіл

## Назви меню; меню застосунку називається app-name

-file-menu = Файл
menu-file = &{ -file-menu }
menu-edit = &Редагування
menu-photo = &Знімок
menu-view = &Перегляд
menu-window = &Вікно
menu-help = &Довідка
menu-panel-tab = Вкладка бічної панелі
menu-interface-size = Розмір інтерфейсу
menu-sort-by = Сортувати за
menu-thumbnail-size = Розмір мініатюр
menu-background = Тло
menu-grid-position = Розташування панелі сітки
menu-open-recent = Відкрити недавні
menu-recent-in = { $name } — { $parent }
menu-services = Служби
menu-hide-app = Приховати { app-name }
menu-hide-others = Приховати інші
menu-show-all = Показати всі
menu-minimise = Згорнути
menu-zoom-window = Масштабувати
menu-bring-all-to-front = Усі вікна на передній план

## Крива

curve-point = Точка { $index }: вхід { $input }%, вихід { $output }%
curve-point-value = Вхід { $input }%, вихід { $output }%
curve-black-point = { $control }: точка чорного
curve-white-point = { $control }: точка білого
curve-keys = Page Up і Page Down вибирають точку, стрілки переміщують її, Insert або + додає точку, Delete видаляє її, а Shift+Delete — усі.
curve-keys-mac = Page Up і Page Down (Fn+↑ і Fn+↓) вибирають точку, стрілки переміщують її, + додає точку, Delete видаляє її, а Shift+Delete — усі.

## Графік смуг еквалайзера тонів; $module — назва модуля

equalizer-name = { $module }, крива
equalizer-tip = Двічі клацніть, щоб скинути криву.
equalizer-keys = Page Up і Page Down вибирають смугу, стрілки переміщують її, Delete скидає її, а Shift+Delete — усі.
equalizer-keys-mac = Page Up і Page Down (Fn+↑ і Fn+↓) вибирають смугу, стрілки переміщують її, Delete скидає її, а Shift+Delete — усі.
equalizer-overflows-shadows = Тіні маски виходять за смугу −8 EV.
equalizer-overflows-highlights = Світлі тони маски виходять за смугу +0 EV.
equalizer-overflows-both = Тіні маски виходять за смугу −8 EV, а світлі тони — за смугу +0 EV.

## Гістограма й інші графіки; $shadows і $highlights — частки пікселів знімка, у відсотках

section-scopes = Графіки
scope-histogram = Гістограма
scope-waveform = Осцилограма
scope-parade = RGB-парад
scope-vectorscope = Вектороскоп
scope-graphs = Графік
scope-histogram-tip = Скільки пікселів має кожну яскравість: чорне ліворуч, біле праворуч.
scope-waveform-tip = Яскравість поперек знімка, зліва направо, як на знімку: темне внизу, світле вгорі.
scope-parade-tip = Осцилограми червоного, зеленого й синього поруч. Де вони збігаються, знімок нейтральний.
scope-vectorscope-tip = Кольори за відтінком, тобто кутом, і за насиченістю, тобто відстанню від центру.
scope-log-scale = Логарифмічна шкала
scope-red = Червоний канал
scope-green = Зелений канал
scope-blue = Синій канал
scope-luminance = Яскравість
scope-show-shadow-clipping = Показати обрізані тіні
scope-show-highlight-clipping = Показати обрізані світла
scope-no-clipping = Без обрізання
scope-clipped-shadows = Обрізано тіней: { $shadows }%
scope-clipped-highlights = Обрізано світел: { $highlights }%
scope-clipped-both = Обрізано тіней: { $shadows }%, світел: { $highlights }%
scope-empty = Ще нічого не виміряно

## Вікно експорту (docs/gui.md, The Export dialog): модуль експорту darktable, назви
## з великої літери лише першого слова. $count — кількість знімків, один знімок — точне
## число 1. $file — назва знімка чи файлу, $source — вихідний raw, $folder — тека, $width
## і $height — пікселі, $problem — що зупиняє експорт.

export-title = Експорт: { photo-count }
export-button = Експортувати { photo-count }
export-close = Закрити
export-hint = Експорт іде у фоні; можна далі працювати.
# Підказка рядка чи групи, показаних вимкненими, бо їх ще не зроблено.
export-later = З'явиться в одній із наступних версій

## Колонка «Шаблони»

export-presets = Шаблони
export-preset-default = Типові налаштування
export-preset-built-in = Вбудований
export-preset-default-tip = Налаштування такі, як у darktable з коробки: вибір скидає їх. Його не можна змінити чи видалити.
export-preset-matches-none = Налаштування не збігаються з жодним шаблоном.
export-presets-later = Власні шаблони з'являться в одній із наступних версій.

## Параметри сховища

export-storage-options = Параметри сховища
export-target-storage = Цільове сховище
export-storage-latex = Шаблон книги LaTeX
export-storage-disk = Файл на диску
export-storage-email = Надіслати поштою
export-storage-gallery = Вебгалерея
export-path = Шлях
export-choose-folder = Вибрати…
# Як екранні читачі називають список у полі шляху.
path-variables = Змінні
export-on-conflict = У разі конфлікту
export-conflict-unique = Створити унікальну назву
export-conflict-overwrite = Перезаписати
export-conflict-changed = Перезаписати, якщо змінено
export-conflict-skip = Пропустити
export-conflict-overwrite-no-alert = Перезапис питає, перш ніж замінити файли, а це питання тут поки не можна показати. Виберіть інший спосіб.

## Що сеанс каже під шляхом і під конфліктом

# $source — raw, $file — куди піде його файл.
export-name = { $source } → { $file }
export-names-more = та ще { $count }, так само
export-no-name = Шлях не дає назви файлу для цього знімка.
export-sequence-added = Знімків: { $count }, змінної в шляху немає: додається _$(SEQUENCE), щоб жоден файл не затер інший.
# $names — змінні darktable, для яких в Apertrust немає значення.
export-not-here = { $names }: з darktable, тут без значення; розкривається в ніщо.
export-conflict-one-number = { $file } уже існує, тому цей файл буде збережено з доданим номером.
export-conflict-one-overwrite = Замінить файл { $file }, який уже існує.
export-conflict-one-skip = Пропустить файл { $file }, який уже існує.
export-conflict-one-unchanged = Пропустить файл { $file }, який уже існує й новіший за правки знімка.
export-conflict-many-number = Уже існують: { $count } з { $total }; їх буде збережено з доданим _01.
export-conflict-many-overwrite = Замінить уже наявні: { $count } з { $total }.
export-conflict-many-skip = Пропустить уже наявні: { $count } з { $total }.

## Що зупиняє експорт (D8)

export-problem-empty = Вкажіть, куди покласти експортовані знімки.
export-problem-nameless = Після “$(” бракує назви змінної. Введіть $(, щоб побачити змінні.
export-problem-unknown = Невідома змінна $({ $name }). Введіть $(, щоб побачити змінні.
export-problem-unknown-near = Невідома змінна $({ $name }). Можливо, ви мали на увазі $({ $suggestion })?
export-fix = Використати $({ $name })
export-problem-unclosed = Бракує “{ $expected }” після “{ $variable }”.
export-problem-no-operation = “{ $found }” після { $variable } не є операцією.
export-problem-too-deep = Змінні вкладено глибше за допустиме: { $depth }.
export-problem-not-connected = Не вдається записати до “{ $folder }”: диск не підключено. Підключіть його або виберіть іншу теку.
export-problem-not-a-folder = Не вдається записати до “{ $folder }”: на цьому місці лежить файл.
export-problem-read-only = Не вдається записати до “{ $folder }”: лише для читання.
export-problem-unreadable = Не вдається записати до “{ $folder }”: { $why }.
export-problem-too-large = { $format }: не більше { $largest } px з одного боку.
export-problem-no-pixels = Розмір менший за піксель: { $width } × { $height } px.
export-problem-failed = Знімок не можна експортувати в такому розмірі: { $why }
# Підказка кнопки «Експорт», поки вона вимкнена; $problem — одне з повідомлень вище.
export-why-path = Спершу виправте шлях: { $problem }
export-why-size = Спершу виправте розмір: { $problem }

## Параметри формату

export-format-options = Параметри формату
export-file-format = Формат файлу
export-format-copy = Копія
export-jpeg-quality = Якість
export-jpeg-subsample = Субдискретизація кольору
export-jpeg-subsample-tip = Субдискретизація кольору для кодувальника JPEG. Авто: за якістю. 4:4:4: немає. 4:4:0: колір удвічі менший по вертикалі. 4:2:2: по горизонталі. 4:2:0: в обох напрямках.
export-subsample-auto = Авто
export-png-bit-depth = Бітова глибина
export-png-compression = Стискання
export-bits-8 = 8 біт
export-bits-16 = 16 біт

## Загальні параметри

export-global-options = Загальні параметри
export-set-size = Розмір
export-mode-pixels = У пікселях (для файлу)
export-mode-cm = У см (для друку)
export-mode-inch = У дюймах (для друку)
export-mode-scale = За масштабом (для файлу)
export-size-pixels = Розмір у пікселях
export-size-cm = Розмір у сантиметрах
export-size-inch = Розмір у дюймах
export-unit-cm = см
export-unit-inch = дюйм
export-width = Ширина
export-height = Висота
export-dpi = Роздільність
export-dpi-unit = dpi
export-equal-to = Це дорівнює { $width } × { $height } px
export-scale = Масштаб
export-scale-tip = Ціле число, десятковий дріб або простий дріб, як-от 1/2. 0 чи порожньо — це 1.
# Розмір активного знімка під рядками; $file — його назва.
export-size = { $file } матиме { $width } × { $height } px.
export-size-capped = { $file } матиме { $width } × { $height } px (без збільшення: «Дозволити збільшення» — Ні).
export-size-full = { $file } матиме { $width } × { $height } px (повний розмір).
export-size-no-limit = { $file } матиме { $width } × { $height } px (повний розмір: межі не задано).
export-upscale = Дозволити збільшення
export-high-quality = Передискретизація високої якості
export-high-quality-tip = Передискретизація високої якості під час експорту: усі операції в повному розмірі, потім зменшення.
export-no = Ні
export-yes = Так
export-masks = Зберігати маски
export-masks-later = Зберігати маски шарами в експортованих зображеннях. Це вміють лише TIFF, OpenEXR і XCF, а шари з'являться в наступній версії.
export-profile = Профіль
export-profile-image = Налаштування зображення
export-profile-tip = Налаштування зображення: власний профіль знімка, у «Колір › Вихідний колірний профіль», як у перегляді. Інший профіль діє лише для цього експорту.
export-intent = Намір
export-intent-perceptual = Сприйняттєвий
export-intent-relative = Відносний колориметричний
export-intent-saturation = Насиченість
export-intent-absolute = Абсолютний колориметричний
export-intent-tip = З матричними профілями й без файлів ICC намір нічого не змінює, тож чекає на файли ICC.
export-style = Стиль
export-style-none = Немає
export-style-later = Стилі з'являться в одній із наступних версій.
export-metadata = Метадані
export-multi-preset = Експорт за кількома шаблонами

## Сповіщення (docs/gui.md, Dialogs and alerts): їх показує сама система. $stoppable --
## скільки з виконуваних завдань зупиняє вихід, $waiting -- на скільки він чекає.

alert-cancel = Скасувати
alert-overwrite-title = Експортувати з перезаписом?
alert-overwrite-message = Експорт замінить файли з тими самими іменами, що вже є. Старі файли повернути не вдасться.
alert-overwrite-button = Перезаписати
alert-overwrite-suppress = Більше не питати
alert-quit-title = Вийти, поки виконується завдання?
alert-quit-message =
    { $stoppable ->
        [0] Apertrust вийде, коли завершаться виконувані завдання.
       *[other]
            { $waiting ->
                [0] Виконувані завдання зупиняться після знімка, що записується, і Apertrust вийде. Уже записані файли залишаться.
               *[other] Завдання, які можна зупинити, зупиняться після знімка, що записується, а Apertrust вийде, коли завершаться решта. Уже записані файли залишаться.
            }
    }
alert-quit-button = Вийти
alert-save-title = Обробку не збережено
alert-save-message = Не вдалося записати обробку { $name } поруч із фото: { $error }
alert-save-button = Спробувати ще раз
alert-quit-save-title = Вийти без останніх змін?
alert-quit-save-message = Не вдалося зберегти останні зміни: { $error }
alert-quit-anyway = Однаково вийти
alert-refused-title = Не вдалося відкрити { $name }
alert-refused-unknown = Тут відкриваються лише теки та raw-знімки.
alert-refused-library = Бібліотеки та проєкти з'являться в одній із наступних версій.
alert-refused-button = Гаразд
