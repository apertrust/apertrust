# Інтерфейс Apertrust українською (docs/gui.md, Translation).
# Переклад повідомлень з en-GB/ui.ftl з тими самими змінними. Чернетка: ще не перевірена
# носієм мови.

app-name = Apertrust

## Загальні дії на смузі інструментів

action-import = Імпорт…
    .toolbar = Імпорт
action-export = Експорт…
    .toolbar = Експорт
action-export-again = Експортувати ще раз
    .toolbar = Експорт ще раз
action-reset = Скинути
    .toolbar = Скинути
action-discard-history = Відкинути історію…
action-undo = Скасувати
    .toolbar = Скасувати
action-redo = Повторити
    .toolbar = Повторити
action-copy-edit = Копіювати обробку
    .toolbar = Копіювати
action-apply-edit = Застосувати обробку
    .toolbar = Застосувати
action-copy-apply-parts = Копіювати або застосувати частину
    .toolbar = Частини обробки
action-save-style = Зберегти обробку як стиль…
action-select-all = Вибрати все
action-select-none = Зняти вибір
action-invert-selection = Інвертувати вибір
action-select-unedited = Вибрати необроблені
action-previous-selection = Попередній вибір
action-settings = Налаштування
    .toolbar = Налаштування
status-saved = Збережено

## Вкладки панелі інструментів

tab-photos = Фото
tab-adjust = Корекція
tab-detail = Деталі
tab-geometry = Геометрія
tab-effects = Ефекти
tab-raw = Raw
tab-base = Базові
tab-color = Колір
tab-correct = Корекція
tab-effect = Ефект
tab-history = Історія
tab-metadata = Метадані
tab-hidden = Приховані
tools-panel-no-hidden = У цьому робочому просторі немає прихованих інструментів.
tools-panel-newer = Це компонування зберегла новіша версія Apertrust, тож зміни в ньому не збережуться
tools-panel-newer-tip = Ця версія ніколи не перезаписує файл робочого простору, який записала новіша версія Apertrust. Переміщені інструменти лишаються на своїх місцях до виходу з програми.
tools-panel-hidden-change = Приховані інструменти змінюють це фото: { $tools }
find-tool-field = Знайти інструмент
find-tool-close = Закрити пошук
find-tool-hint = Введіть назву інструмента.
find-tool-none = Жоден інструмент не відповідає «{ $text }».
find-tool-count =
    { $count ->
        [one] { $count } інструмент
        [few] { $count } інструменти
        [many] { $count } інструментів
       *[other] { $count } інструмента
    }
find-tool-lives-in = у вкладці { $tab }
find-tool-lives-hidden = прихований
workspace-default = Типовий
workspace-essentials = Основне
panel-tab-later = { $tab }: з'явиться в одній із наступних версій
tools-panel-tabs = Вкладки панелі інструментів
panel-more-tabs = Інші вкладки
panel-tab-options = Параметри вкладок
panel-show-tab-names = Показувати назви вкладок
panel-customise-tabs = Налаштувати вкладки…
tools-panel-width = Ширина панелі інструментів
tools-panel-position = Розташування

## Кнопки в заголовку розділу; $section — назва розділу

section-reset = Скинути: { $section }
section-presets = Пресети: { $section }
section-options = Параметри: { $section }
section-options-none = Параметрів поки немає
section-options-later = З'явиться в одній із наступних версій
section-toggle = Відкрити або закрити: { $section }. Утримуйте H над розділом, щоб побачити знімок без нього.
section-tabs = Вкладки: { $section }
section-tab-later = З'явиться в одній із наступних версій
section-not-applied = Не застосовано
section-moved = { $tool }: { $at }-е місце з { $of } на вкладці «{ $tab }»
section-pinned = { $tool }: закріплено на вкладці «{ $tab }»
section-unpinned = { $tool }: відкріплено на вкладці «{ $tab }»
section-moved-to = { $tool }: переміщено на вкладку «{ $tab }»
section-hidden = { $tool }: приховано
section-shown = { $tool }: показано на вкладці «{ $tab }»
section-about-kind = Тип
section-about-works-on = Працює з
section-about-measures = Вимірює
section-about-input = На вході
section-about-process = Обробка
section-about-output = На виході
section-about-row = { $label }: { $value }
choice-option-later = З'явиться в одній із наступних версій
choice-option-unavailable = Недоступно для цього знімка

## Згортана група в інструменті, згортаний розділ darktable

collapsible-group-set = Змінено: { $count }
collapsible-group-tip = { $group }: змінено { $count }
collapsible-group-active = { $group } (активне)

## Рядок пресетів в інструменті, як Режим балансу білого

preset-custom = Власні
preset-option-unavailable = Недоступно для цього знімка

## A tool's presets

presets-default = Типові налаштування
presets-save = Зберегти як шаблон…
presets-update = Оновити «{ $name }»
presets-rename = Перейменувати…
presets-delete = Видалити…
presets-choose-first = Спершу виберіть шаблон
presets-no-alert = Ця дія спершу питає, а показати запитання тут поки неможливо.
presets-parked-newer = Створено новішою версією Apertrust
presets-parked-tool = Створено для новішої версії інструмента «{ $tool }»: версія { $version }
presets-save-title = Зберегти як шаблон
presets-rename-title = Перейменувати шаблон
presets-name = Назва
presets-new-name = { $tool } { $number }
presets-description = Опис
presets-description-hint = Необов’язково
presets-save-button = Зберегти
presets-rename-button = Перейменувати
presets-name-empty = Шаблону потрібна назва
presets-name-taken = Інструмент «{ $tool }» уже має шаблон із такою назвою
preset-export-for-sharing = Для поширення
preset-export-full-size-jpeg = JPEG у повному розмірі

## Створення стилю

styles-create-title = Створити стиль
styles-tools = Інструменти
styles-tool-one-photo = Зазвичай стосується лише одного знімка
styles-name-empty = Стилю потрібна назва
styles-name-taken = Стиль із такою назвою вже є
styles-no-tool = Виберіть хоча б один інструмент
styles-create-button = Створити
style-created = Стиль «{ $name }» створено
style-not-created = Не вдалося створити стиль «{ $name }»: { $error }

## Інструмент «Стилі» на вкладці «Знімки»

styles-find = Знайти стиль
styles-empty = Стилів ще немає.
styles-applied = Застосовані стилі
styles-applied-item = { $name }, застосовано
styles-tree = Стилі
styles-no-photo = Відкрийте знімок, щоб застосувати стиль
styles-view-only = Знімок відкрито лише для перегляду
styles-parked = Створено новішою версією Apertrust
styles-parked-retired = Містить лише інструменти, яких Apertrust більше не має
styles-row-menu = Меню «{ $name }»
styles-apply = Застосувати
styles-clear = Прибрати

styles-apply-photos =
    { $count ->
        [one] Застосувати до { $count } знімка
        [few] Застосувати до { $count } знімків
        [many] Застосувати до { $count } знімків
       *[other] Застосувати до { $count } знімка
    }
batch-applied =
    { $count ->
        [one] «{ $style }» застосовано до { $count } знімка
        [few] «{ $style }» застосовано до { $count } знімків
        [many] «{ $style }» застосовано до { $count } знімків
       *[other] «{ $style }» застосовано до { $count } знімка
    }
batch-applied-some = «{ $style }» застосовано до { $changed } із { $count } знімків
batch-taken-back =
    { $count ->
        [one] «{ $style }» скасовано на { $count } знімку
        [few] «{ $style }» скасовано на { $count } знімках
        [many] «{ $style }» скасовано на { $count } знімках
       *[other] «{ $style }» скасовано на { $count } знімках
    }
batch-taken-back-some = «{ $style }» скасовано на { $changed } із { $count } знімків
batch-put-back =
    { $count ->
        [one] «{ $style }» повернено на { $count } знімку
        [few] «{ $style }» повернено на { $count } знімках
        [many] «{ $style }» повернено на { $count } знімках
       *[other] «{ $style }» повернено на { $count } знімках
    }
batch-put-back-some = «{ $style }» повернено на { $changed } із { $count } знімків
batch-cancelled = { $said }; скасовано
batch-skipped =
    { $said }; { $photo }: { $why ->
        [view-only] лише для перегляду
        [not-on-disk] немає на цьому комп’ютері
        [unreadable] не вдається прочитати
        [refused] не приймає цей стиль
        [changing] постійно змінюється
        [failed] не вдалося зберегти
       *[changed] змінено відтоді
    }{ $more ->
        [0] { "" }
       *[other] , і ще { $more }
    }
styles-rename = Перейменувати стиль…
styles-delete = Видалити стиль…
styles-mode = Режим
styles-mode-append = Додати
styles-mode-replace = Замінити
styles-import = Імпортувати стиль…
styles-export = Експортувати стиль…
styles-later = З’явиться в наступній версії
styles-left-out = { $count ->
    [one] { $count } інструмент стилю «{ $name }» пропущено: { $reason }
    [few] { $count } інструменти стилю «{ $name }» пропущено: { $reason }
   *[other] { $count } інструментів стилю «{ $name }» пропущено: { $reason }
}
styles-reason-newer = створено новішою версією Apertrust
styles-reason-retired = інструмент, який Apertrust більше не має
styles-find-clear = Очистити пошук
styles-applied-state = застосовано
styles-rename-title = Перейменувати стиль
styles-rename-button = Перейменувати
style-not-renamed = Не вдалося перейменувати стиль «{ $name }»: { $error }
style-not-deleted = Не вдалося видалити стиль «{ $name }»: { $error }
styles-built-in = Вбудовані стилі
styles-custom = Власні стилі
styles-copy = Копіювати до власних стилів…
styles-copy-title = Копіювати до власних стилів
styles-copy-button = Копіювати
styles-copy-name = { $name } (копія)
style-not-copied = Не вдалося скопіювати стиль «{ $name }»: { $error }
style-builtin-bw-luminance = Чорно-біле|Чорно-біле
style-builtin-bw-hp5 = Чорно-біле|HP5+
style-builtin-bw-acros-100 = Чорно-біле|Acros 100
style-builtin-natural-colour = Природні кольори
style-builtin-vibrant-colour = Яскраві кольори
style-builtin-punchy = Виразний
style-builtin-open-shadows-highlights = Відкриті тіні й світла
style-builtin-lomo = Ломо

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
theme-grey = Сіра
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

## Смуга інструментів

toolbar = Смуга інструментів
toolbar-menu = Меню
toolbar-more = Інші дії
action-toolbar-labels = Підписи на смузі інструментів
action-customise-toolbar = Налаштувати смугу інструментів…
action-reset-toolbar = Скинути смугу інструментів
action-toolbar-hide = Сховати елемент смуги інструментів
toolbar-menu-hide = Сховати «{ $item }»
toolbar-item-hidden = { $item }: сховано
action-show-menu-button = Показати кнопку меню
toolbar-customise-later = Налаштування смуги інструментів з'явиться в одній із наступних версій
status-saving = Збереження…
status-saving-tip = Обробку цього фото ще записують поруч із ним
status-not-saved = Не збережено
status-not-saved-reason = Не збережено: { $error }
status-view-only = Лише перегляд
window-minimise = Згорнути
window-maximise = Розгорнути
window-restore = Відновити
window-close = Закрити

## Завдання: індикатор у кінці смуги інструментів і вікно, яке він відкриває

jobs-name = Завдання
action-show-jobs = Завдання
    .toolbar = Завдання
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
jobs-copy-error = Копіювати помилку
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

job-apply-style = Застосування «{ $style }» · { photo-count }
job-apply-style-now = Готово { $done } із { $total } · зараз { $photo }
job-apply-style-outcome =
    { $count ->
        [one] «{ $style }» застосовано до { $changed } із { $count } знімка
        [few] «{ $style }» застосовано до { $changed } із { $count } знімків
        [many] «{ $style }» застосовано до { $changed } із { $count } знімків
       *[other] «{ $style }» застосовано до { $changed } із { $count } знімка
    }
job-undo-batch = Скасування «{ $style }» · { photo-count }
job-redo-batch = Повернення «{ $style }» · { photo-count }
job-undo-batch-outcome =
    { $count ->
        [one] «{ $style }» скасовано на { $changed } із { $count } знімка
        [few] «{ $style }» скасовано на { $changed } із { $count } знімків
        [many] «{ $style }» скасовано на { $changed } із { $count } знімків
       *[other] «{ $style }» скасовано на { $changed } із { $count } знімка
    }
job-redo-batch-outcome =
    { $count ->
        [one] «{ $style }» повернено на { $changed } із { $count } знімка
        [few] «{ $style }» повернено на { $changed } із { $count } знімків
        [many] «{ $style }» повернено на { $changed } із { $count } знімків
       *[other] «{ $style }» повернено на { $changed } із { $count } знімка
    }

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
job-cause-binding = змінений інструмент не вдалося застосувати до фото
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

## Смуга переглядача, над знімком

viewer-bar = Смуга переглядача
photo-show-selected = Показати всі вибрані знімки
photo-proof-margin = Поле перевірки
photo-before-after = До і після
guides-show-label = Показати напрямні
guides-show-full = Показати напрямні
guides-show-promise = Показати накладені напрямні лінії, коли цей інструмент має фокус.
guides-settings-full = Змінити глобальні налаштування напрямних ліній
guides-settings-promise = Ці налаштування застосовуються глобально і впливатимуть на будь-який інструмент, який показує накладення напрямних.
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
photo-zoom-tip = Масштаб: зведення пальців або { $command }+прокручування; Z — 100%; клацання середньою кнопкою — по черзі 100%, 200% і «За розміром». Прокручування рухає збільшений знімок, Shift+прокручування — убік
photo-zoom-tip-exact = На 100% екран показує саме те, яким буде експорт у повному розмірі. Масштаб: зведення пальців або { $command }+прокручування; Z — 100%; клацання середньою кнопкою — по черзі 100%, 200% і «За розміром». Прокручування рухає збільшений знімок, Shift+прокручування — убік
photo-view = Знімок, { $name }
photo-view-value = { $scale }, { $left }–{ $right }% по ширині, { $top }–{ $bottom }% по висоті
layer-image = Знімок

## Переглядач

photo-opening = Відкриваю { $name }…
photo-rendering = Рендеринг…
controls-opening = Відкриваю { $name }…
edit-not-saved = Обробку { $name } не збережено: { $error }
photo-view-only = Оброблено в новішому Apertrust, { $version }: відкрито лише для перегляду
photo-view-only-unknown = Оброблено в новішому Apertrust: відкрито лише для перегляду
photo-view-only-unreadable = Частину історії цієї обробки неможливо прочитати: відкрито лише для перегляду
photo-view-only-unkept = Частину історії цієї обробки неможливо зберегти: відкрито лише для перегляду
edit-not-saved-unreadable = інша програма змінила її файл, і цей Apertrust не може його прочитати
edit-not-saved-view-only = інша програма змінила її файл, і тепер він відкритий лише для перегляду
edit-not-saved-still-changing = інша програма змінювала її файл, поки фото закривалося
edit-move-not-kept = Інша програма змінила історію { $name }: ваше останнє скасування, повтор, перехід, стиснення, обрізання чи відкидання історії не збережено
photo-shutter = { $value } с
photo-aperture = f/{ $value }
photo-focal-length = { $value } мм
photo-iso = ISO { $value }

## Діалог відкриття, з меню Файл › Відкрити…

open-title = Відкрити
open-filter = Raw-знімки

## Оглядач і його комірки

browser = Оглядач
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
# Опис комірки для екранних читачів, після назви файлу: у файлі немає зображення, яке
# Apertrust може прочитати, і знімок ще не відрендерено.
grid-cell-no-preview = Ще немає мініатюри
# Останній рядок підказки тієї ж комірки.
grid-cell-no-preview-tip = У цьому файлі ще немає мініатюри. Apertrust створить її, коли ви відкриєте знімок.
colour-label-red = Червоний
colour-label-yellow = Жовтий
colour-label-green = Зелений
colour-label-blue = Синій
colour-label-purple = Фіолетовий
caption-reject = Відхилити
caption-unreject = Скасувати відхилення
caption-rating = Оцінка
caption-clear-rating = Зняти оцінку
caption-label-red = Червона мітка
caption-label-yellow = Жовта мітка
caption-label-green = Зелена мітка
caption-label-blue = Синя мітка
caption-label-purple = Фіолетова мітка
caption-tip = { $action } ({ $key })
caption-this-photo = { $action }, лише цей знімок
browser-height = Висота оглядача
grid-rows =
    { $count ->
        [one] { $count } рядок
        [few] { $count } рядки
        [many] { $count } рядків
       *[other] { $count } рядка
    }
browser-width = Ширина оглядача
grid-columns =
    { $count ->
        [one] { $count } стовпець
        [few] { $count } стовпці
        [many] { $count } стовпців
       *[other] { $count } стовпця
    }
browser-bar-options = Параметри оглядача
browser-position = Розташування
browser-bar = Смуга оглядача
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
browser-count =
    { $selected ->
        [0] { $total }
       *[other] { $selected }/{ $total }
    }
browser-count-thousands = { $thousands }к
browser-selected-count =
    { $total ->
        [one] Вибрано { $selected } із { $total } знімка
        [few] Вибрано { $selected } із { $total } знімків
        [many] Вибрано { $selected } із { $total } знімків
       *[other] Вибрано { $selected } із { $total } знімка
    }
browser-photo-count =
    { $total ->
        [one] { $total } знімок
        [few] { $total } знімки
        [many] { $total } знімків
       *[other] { $total } знімка
    }

## Що відкрито, за видом

open-kind-folder = Тека: { $name }

## Режим теки: Файл › Відкрити теку…, розділ «Теки» вкладки «Бібліотека» та порожні стани

action-open-folder = Відкрити теку…
    .toolbar = Відкрити теку
open-folder-title = Відкрити теку
section-collections = Колекції
    .search = Колекції
section-styles = Стилі
    .search = стилі вигляд
collections-folders = Теки
section-export = Експорт
    .search = Експорт
collections-open-name = Що відкрито
collections-no-folder = Жодної теки не відкрито
collections-recents-none = Немає нещодавніх тек
collections-recent-missing = { $label }, не знайдено
collections-try-again = Спробувати ще раз
collections-locate = Знайти…
collections-remove = Прибрати зі списку
collections-locate-title = Знайти теку
collections-plus = Створити або відкрити
action-new-library = Нова бібліотека…
collections-folder-menu = Меню теки { $name }
collections-open-row = Відкрити
collections-open-this-folder = Відкрити цю теку
collections-reading = Читання…
collections-unreadable = Не вдалося прочитати. Спробувати ще раз
folders-reading = Читання { $name }…
folders-recent-missing = Цю теку не вдалося відкрити. { $reason }
folders-recent-gone = Цієї теки там більше немає.
folder-reading = Читання { $folder }…
folder-empty = Немає доступних фото
folder-empty-hint = Виберіть підтеку в розділі «Колекції».
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
start-open = Відкрити
start-copyright = © 2026
start-tagline =
    Вільний, недеструктивний
    редактор raw-знімків
start-failed = Минулого разу { app-name } не запустився з { $card } ({ $api }), тому використано автоматичний вибір
start-failed-card = Минулого разу { app-name } не запустився з { $card }, тому використано автоматичний вибір
start-failed-api = Минулого разу { app-name } не запустився з { $api }, тому використано автоматичний вибір
start-failed-on = Минулого разу { app-name } не запустився з { $card } ({ $api }), тому використано автоматичний вибір з { $other }
start-failed-api-on = Минулого разу { app-name } не запустився з { $api }, тому використано автоматичний вибір з { $other }
start-failed-automatic = Минулого разу { app-name } не запустився з автоматичним вибором, тому його запущено на ЦП, а це повільніше
start-failed-automatic-again = Минулого разу { app-name } не запустився з автоматичним вибором, а з цим графічним API немає адаптера ЦП, тому знову використано автоматичний вибір
start-failed-keep-automatic = Залишити автоматичний вибір
start-failed-keep-automatic-on = Залишити автоматичний вибір з { $api }
start-failed-keep-cpu = Залишити ЦП
start-failed-retry = Спробувати знову
start-failed-close = Закрити
start-failed-overridden = { $flag } задає відеокарту або графічний API для цього запуску, тому нічого не збережено. Запустіть { app-name } без нього, щоб зберегти це.

## Налаштування

settings-close = Закрити
settings-find = Знайти налаштування
settings-overridden = Задано через { $flag } для цього запуску
settings-requires = Вимкнено, поки вимкнено «{ $setting }»
settings-changed = Змінено порівняно з типовим значенням
settings-default-on = Увімкнено
settings-default-off = Вимкнено
settings-default-empty = порожнього
settings-reset = Скинути
settings-reset-to = Скинути до { $value }
settings-restart-note = після перезапуску
settings-value-with-unit = { $value } { $unit }
settings-number-range = від { $min } до { $max }
settings-number-range-from = { $min } або більше
settings-number-range-up-to = до { $max }
settings-number-range-above = більше за { $min }
settings-number-range-above-to = більше за { $min }, до { $max }
settings-number-decrease = Зменшити: { $control }
settings-number-increase = Збільшити: { $control }
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
settings-group-tools-panel = Панель інструментів
settings-group-hardware-acceleration = Апаратне прискорення
settings-group-diagnostics = Діагностика
settings-group-cache = Кеш
setting-theme-label = Тема
setting-theme-option-dark = Темна
setting-theme-option-grey = Сіра
setting-theme-option-light = Світла
setting-theme-option-high-contrast = Висока контрастність
setting-locale-label = Мова
setting-locale-default = мови системи
setting-scale-label = Розмір інтерфейсу
setting-sliders-marker-label = Маркер повзунка
setting-sliders-marker-option-triangle = Трикутник
setting-sliders-marker-option-circle = Коло
setting-sliders-marker-option-square = Квадрат
setting-toolbar-menu-button-label = Кнопка меню
setting-toolbar-menu-button-option-icon = Значок { app-name }
setting-toolbar-menu-button-option-menu = Меню
setting-toolbar-labels-label = Підписи на смузі інструментів
setting-library-reopen-last-label = Відкривати те, що було відкрито востаннє
setting-library-reopen-last-help = Відкриває теку, з якою ви працювали, коли { app-name } закрився, разом з її фото. Файл або тека, відкриті з файлового менеджера чи термінала, відкриваються замість неї.
setting-export-ask-before-overwrite-label = Питати перед перезаписом експортованого файлу
setting-history-ask-before-discard-label = Питати перед відкиданням історії знімка
setting-browser-square-thumbnails-label = Квадратні мініатюри
setting-library-raw-thumbnails-from-label = Мініатюри з raw від розміру
setting-library-raw-thumbnails-from-help = Для знімків без змін: мініатюри, більші за цей розмір, рендеряться з raw-файлу в { app-name }, а не беруться з вбудованого JPEG камери. Повільніше. Знімки зі змінами або показані в { app-name } щонайменше 300 мс завжди показують рендер { app-name }.
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
setting-tools-panel-tab-names-label = Показувати назви вкладок
setting-tools-panel-wheel-scrolls-panel-label = Коліщатко миші прокручує панель інструментів
setting-tools-panel-wheel-scrolls-panel-help = Увімкнено: коліщатко змінює повзунок чи список під вказівником лише з натиснутою Ctrl, на Mac — Option. Вимкнено, як у darktable: змінює їх і без клавіші, а повзунок із нею — на десяту частину кроку.
setting-engine-adapter-label = Відеокарта
setting-engine-backend-label = Графічний API
setting-engine-backend-option-auto = Автоматично
setting-engine-backend-option-dx12 = Direct3D 12
setting-engine-backend-option-vulkan = Vulkan
setting-engine-memory-budget-label = Бюджет пам'яті
settings-gpu-automatic = Автоматично
settings-gpu-cpu = ЦП (без відеокарти), повільно
settings-gpu-cpu-short = ЦП
settings-gpu-cpu-missing = Не знайдено з { $api }
settings-gpu-cpu-missing-unnamed = Не знайдено з цим графічним API
settings-gpu-not-found = Відеокарту не знайдено: використано автоматичний вибір
settings-gpu-not-found-item = Не знайдено на цьому комп'ютері
settings-gpu-dropped = { $card }: недоступно з { $api }, встановлено автоматичний вибір
settings-gpu-dropped-unnamed = { $card }: недоступно з цим графічним API, встановлено автоматичний вибір
settings-gpu-listing = Пошук відеокарт…
settings-gpu-not-listed = Не вдалося отримати список відеокарт
settings-gpu-no-adapter = З { $api } відеокарту не знайдено: використано автоматичний вибір
settings-gpu-no-adapter-unnamed = Відеокарту не знайдено: використано автоматичний вибір
settings-gpu-in-use = Використовується
settings-gpu-in-use-value = { $card }, { $api }
settings-gpu-api-metal = Metal
settings-gpu-api-gl = OpenGL
settings-gpu-software = Відеокарта не використовується: { app-name } обробляє фото на процесорі, а це повільніше
settings-gpu-driver = Може допомогти оновлення графічного драйвера
settings-restart-waiting = Ваш вибір набуде чинності після перезапуску { app-name }
settings-restart-now = Перезапустити зараз
settings-restart-tip = Закриває { app-name } і відкриває знову, з цією текою та фото
settings-restart-failed = Не вдається знову запустити { app-name }: { $error }
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
about-copy-version = Клацніть, щоб скопіювати версію
about-version-copied = Версію скопійовано
about-copyright = Авторське право © 2026 розробники { app-name }.
about-free-software = Вільне програмне забезпечення за ліцензією GNU General Public License версії 3 або пізнішої.
about-credits = Подяки
about-licence = Ліцензія
about-back = Назад

## Дії, як їх називає карта клавіш

action-open = Відкрити…
    .toolbar = Відкрити
action-view-edit = Перейти до обробки
action-show-collections = Колекції
    .toolbar = Колекції
action-panel-tab-1 = Вкладка панелі інструментів 1
action-panel-tab-2 = Вкладка панелі інструментів 2
action-panel-tab-3 = Вкладка панелі інструментів 3
action-panel-tab-4 = Вкладка панелі інструментів 4
action-panel-tab-5 = Вкладка панелі інструментів 5
action-panel-tab-6 = Вкладка панелі інструментів 6
action-panel-tab-7 = Вкладка панелі інструментів 7
action-panel-tab-8 = Вкладка панелі інструментів 8
action-panel-tab-9 = Вкладка панелі інструментів 9
action-panel-tab-10 = Вкладка панелі інструментів 10
action-panel-tab-11 = Вкладка панелі інструментів 11
action-panel-tab-12 = Вкладка панелі інструментів 12
action-panel-tab-hidden = Вкладка «Приховані»
action-find-tool = Знайти інструмент…
action-tools-panel-show-hidden = Показувати приховані інструменти
action-section-pin = Закріпити інструмент
action-section-unpin = Відкріпити інструмент
action-section-move-up = Перемістити інструмент вище
action-section-move-down = Перемістити інструмент нижче
action-section-move-to-tab-1 = Перемістити інструмент на вкладку 1
action-section-move-to-tab-2 = Перемістити інструмент на вкладку 2
action-section-move-to-tab-3 = Перемістити інструмент на вкладку 3
action-section-move-to-tab-4 = Перемістити інструмент на вкладку 4
action-section-move-to-tab-5 = Перемістити інструмент на вкладку 5
action-section-move-to-tab-6 = Перемістити інструмент на вкладку 6
action-section-move-to-tab-7 = Перемістити інструмент на вкладку 7
action-section-move-to-tab-8 = Перемістити інструмент на вкладку 8
action-section-move-to-tab-9 = Перемістити інструмент на вкладку 9
action-section-move-to-tab-10 = Перемістити інструмент на вкладку 10
action-section-move-to-tab-11 = Перемістити інструмент на вкладку 11
action-section-move-to-tab-12 = Перемістити інструмент на вкладку 12
action-section-hide = Прибрати інструмент із вкладок
action-section-show = Показати інструмент на вкладці
action-section-reset = Скинути
action-section-copy-settings = Копіювати налаштування
action-section-paste-settings = Вставити налаштування
action-scale-up = Збільшити інтерфейс
action-scale-down = Зменшити інтерфейс
action-scale-reset = Звичайний розмір інтерфейсу
action-next-region = Наступна панель
action-previous-region = Попередня панель
action-full-screen = На весь екран
    .toolbar = На весь екран
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
action-restart = Перезапустити
action-peek = Показати знімок без розділу під вказівником
action-next-photo = Наступний знімок
    .toolbar = Наступний
action-previous-photo = Попередній знімок
    .toolbar = Попередній
action-rating-0 = Без зірок
    .toolbar = Без зірок
action-rating-1 = 1 зірка
    .toolbar = 1 зірка
action-rating-2 = 2 зірки
    .toolbar = 2 зірки
action-rating-3 = 3 зірки
    .toolbar = 3 зірки
action-rating-4 = 4 зірки
    .toolbar = 4 зірки
action-rating-5 = 5 зірок
    .toolbar = 5 зірок
action-reject = Відхилити
    .toolbar = Відхилити
action-label-red = Червона мітка
    .toolbar = Червона
action-label-yellow = Жовта мітка
    .toolbar = Жовта
action-label-green = Зелена мітка
    .toolbar = Зелена
action-label-blue = Синя мітка
    .toolbar = Синя
action-label-purple = Фіолетова мітка
    .toolbar = Фіолетова
action-label-clear = Зняти мітки
    .toolbar = Без міток
action-zoom-in = Збільшити
action-zoom-out = Зменшити
action-zoom-fit = За розміром
    .toolbar = За розміром
action-zoom-100 = Масштаб 100%
    .toolbar = 100%
action-zoom-small = Масштаб: малий
action-zoom-fill = Масштаб: заповнити
action-zoom-50 = Масштаб 50%
action-zoom-200 = Масштаб 200%
action-zoom-400 = Масштаб 400%
action-zoom-800 = Масштаб 800%
action-zoom-1600 = Масштаб 1600%
action-zoom-in-all = Збільшити всі знімки
action-zoom-out-all = Зменшити всі знімки
action-zoom-fit-all = Усі знімки за розміром
action-zoom-100-all = Усі знімки на 100%
action-pan-left = Зсунути вигляд ліворуч
action-pan-right = Зсунути вигляд праворуч
action-pan-up = Зсунути вигляд угору
action-pan-down = Зсунути вигляд униз
action-pan-page-left = Зсунути вигляд на екран ліворуч
action-pan-page-right = Зсунути вигляд на екран праворуч
action-pan-page-up = Зсунути вигляд на екран угору
action-pan-page-down = Зсунути вигляд на екран униз
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
action-browser-position-left = Ліворуч
action-browser-position-right = Праворуч
action-browser-position-bottom = Унизу
action-tools-panel-position-left = Ліворуч
action-tools-panel-position-right = Праворуч
action-show-labels = Показувати підписи
action-background-black = Чорне
action-background-very-dark = Дуже темне
action-background-dark = Темне
action-background-medium = Середнє
action-background-light = Світле
action-background-white = Біле
action-guides = Напрямні
    .toolbar = Напрямні
action-guides-grid = Сітка
action-guides-rules-of-thirds = Правило третин
action-guides-metering = Мірні лінії
action-guides-perspective = Перспектива
action-guides-diagonal-method = Діагоналі
action-guides-harmonious-triangles = Гармонійні трикутники
action-guides-golden-sections = Золоті перетини
action-guides-golden-spiral = Золота спіраль
action-guides-golden-spiral-sections = Золоті спіральні перетини
action-guides-golden-mean = Золотий перетин (всі напрямні)
action-guides-flip-none = Немає
action-guides-flip-horizontally = Горизонтально
action-guides-flip-vertically = Вертикально
action-guides-flip-both = Обидва
guides-flip-unavailable = Віддзеркалюються лише гармонійні трикутники та золоті напрямні
toolbar-guides-menu = Клацніть правою кнопкою, щоб вибрати тип напрямних
key-space = Пробіл

## Назви меню; меню застосунку називається app-name

-file-menu = Файл
menu-file = &{ -file-menu }
menu-edit = &Редагування
# В is Вікно's.
menu-select = В&ибір
menu-photo = &Знімок
menu-view = &Перегляд
menu-window = &Вікно
menu-help = &Довідка
menu-tools-panel-tab = Вкладка панелі інструментів
menu-workspace = Робочий простір
menu-panels = Панелі
menu-section-move-to-tab = Перемістити інструмент на вкладку
menu-section-show-in-tab = Показати інструмент на вкладці
tab-add-tool = Додати інструмент
tab-add-tool-none = Усі інструменти вже на цій вкладці
section-hide-tip = Прибирає інструмент «{ $tool }» із вкладок цього робочого простору. Його зміни лишаються; «Показувати приховані інструменти» показує його.
section-hide-export = Експорт є в кожному робочому просторі
section-move-up-first =
    { $list ->
        [pinned] Інструмент «{ $tool }» уже вгорі серед закріплених
       *[other] Інструмент «{ $tool }» уже вгорі
    }
section-move-down-last =
    { $list ->
        [pinned] Інструмент «{ $tool }» уже внизу серед закріплених
       *[other] Інструмент «{ $tool }» уже внизу
    }
menu-interface-size = Розмір інтерфейсу
menu-sort-by = Сортувати за
menu-thumbnail-size = Розмір мініатюр
menu-background = Тло
menu-guides = Напрямні
menu-guides-flip = Віддзеркалити
menu-rating = Оцінка
menu-colour-label = Кольорова мітка
menu-browser-position = Розташування оглядача
menu-tools-panel-position = Розташування панелі інструментів
menu-open-recent = Відкрити недавні
menu-recent-in = { $name } — { $parent }
menu-undo-step = Скасувати: { $step }
menu-redo-step = Повторити: { $step }
history-on-photos =
    { $count ->
        [one] { $step } на { $count } знімку
        [few] { $step } на { $count } знімках
        [many] { $step } на { $count } знімках
       *[other] { $step } на { $count } знімках
    }
menu-export-photos =
    { $count ->
        [one] Експортувати { $count } знімок…
        [few] Експортувати { $count } знімки…
        [many] Експортувати { $count } знімків…
       *[other] Експортувати { $count } знімка…
    }
menu-export-photos-again =
    { $count ->
        [one] Експортувати ще раз { $count } знімок
        [few] Експортувати ще раз { $count } знімки
        [many] Експортувати ще раз { $count } знімків
       *[other] Експортувати ще раз { $count } знімка
    }
menu-services = Служби
menu-hide-app = Приховати { app-name }
menu-hide-others = Приховати інші
menu-show-all = Показати всі
menu-minimise = Згорнути
menu-zoom-window = Масштабувати
menu-bring-all-to-front = Усі вікна на передній план

## Контекстні меню

context-menu-photo = Знімок
context-menu-thumbnail = Мініатюра
context-menu-step = Крок { $number }
context-menu-job = Завдання

## Історія обробки: назва кроку

history-reset = Скидання
history-tools =
    { $count ->
        [one] { $tool } та ще { $count } інструмент
        [few] { $tool } та ще { $count } інструменти
        [many] { $tool } та ще { $count } інструментів
       *[other] { $tool } та ще { $count } інструмента
    }
history-go-to-step = Перехід до кроку
history-compress = Стиснення історії
history-truncate = Обрізання історії
history-discard = Відкидання історії
history-unreject = Скасування відхилення
history-preset = Шаблон { $name }
history-style = Стиль { $style }
history-clear-style = Прибрати { $style }
history-styles-applied = Застосовані стилі
history-default-settings = Типові налаштування
history-options-compress = Стиснути історію
history-options-truncate = Обрізати історію
history-compress-tip = Залишає останній крок кожного інструмента до вибраного й вилучає решту. Знімок не змінюється, окрім як на «Оригіналі»: там уся історія зникає, і знімок починає знову з типових кроків.
history-truncate-tip = Вилучає кроки після вибраного, решту залишає як є. Знімок не змінюється, окрім як на «Оригіналі»: там уся історія зникає, і знімок починає знову з типових кроків.
history-options-style = Створити стиль…
history-options-style-tip = Зберігає обробку цього знімка або вибрані її інструменти як стиль для інших знімків.
history-discard-tip = Очищає історію й повертає знімок до того, яким він відкрився вперше.
history-discard-tip-unreadable = Починає історію наново від знімка, яким він є зараз. Те, що не вдається прочитати, буде втрачено.
history-discard-no-alert = Відкидання історії спершу питає, а це питання тут поки не можна показати. Щоб відкидати без питання, вимкніть у Налаштуваннях «Питати перед відкиданням історії знімка».
history-discard-no-alert-unreadable = Відкидання цієї історії спершу питає, адже те, що не вдається прочитати, уже не повернути, а це питання тут поки не можна показати.
history-step-go-to = Перейти до кроку
history-step-go-to-tip = Показує знімок таким, яким він був на цьому кроці. Кроки після нього залишаються, доки їх не замінить ваше наступне редагування.
history-step-show-tool = Показати інструмент
history-step-show-tool-tip = Відкриває інструмент цього кроку на панелі інструментів, як і клацання з Shift по кроку.
history-step-compress = Стиснути до цього кроку
history-step-compress-tip = Переходить до цього кроку, тоді залишає останній крок кожного інструмента до нього й вилучає решту.
history-step-truncate = Обрізати після цього кроку
history-step-truncate-tip = Переходить до цього кроку, тоді вилучає кроки після нього, а решту залишає як є.

## Розділ «Історія» на вкладці «Історія» панелі інструментів

section-history = Історія
    .search = Історія
history-reset-edit = Скинути обробку
history-step-name = Крок { $number }: { $name }
history-original = Оригінал
history-original-tip = Знімок без жодного з цих кроків
history-source-you = Ви
history-source-auto = Авто
history-source-style = Стиль
history-source-assistant = Асистент
history-source-default = Типово
history-said-change = { $control }, з { $before } на { $after }
history-said-change-alone = з { $before } на { $after }
history-line-controls =
    { $count ->
        [one] { $count } параметр
        [few] { $count } параметри
        [many] { $count } параметрів
       *[other] { $count } параметра
    }
history-step-name-change = Крок { $number }: { $name }, { $change }
history-group-tools =
    { $count ->
        [one] { $count } інструмент
        [few] { $count } інструменти
        [many] { $count } інструментів
       *[other] { $count } інструмента
    }
history-group-name = Кроки з { $first } по { $last }: { $name }, { $tools }
history-defaults = Типові
history-defaults-tools =
    { $count ->
        [one] { $count } інструмент, що працює для кожного фото
        [few] { $count } інструменти, що працюють для кожного фото
        [many] { $count } інструментів, що працюють для кожного фото
       *[other] { $count } інструмента, що працюють для кожного фото
    }
history-line-name = { $name }, { $change }
history-day-today = Сьогодні
history-day-yesterday = Учора, { $date }
history-day-unknown = Без дати
history-undone =
    { $count ->
        [1] ↑ 1 скасований · наступна зміна його замінить
        [one] ↑ { $count } скасований · наступна зміна їх замінить
        [few] ↑ { $count } скасовані · наступна зміна їх замінить
        [many] ↑ { $count } скасованих · наступна зміна їх замінить
       *[other] ↑ { $count } скасованого · наступна зміна їх замінить
    }
history-undone-said =
    { $count ->
        [1] 1 скасований: наступна зміна його замінить
        [one] { $count } скасований: наступна зміна їх замінить
        [few] { $count } скасовані: наступна зміна їх замінить
        [many] { $count } скасованих: наступна зміна їх замінить
       *[other] { $count } скасованого: наступна зміна їх замінить
    }
history-kept = { $control }: закріплено
history-released = { $control }: відпущено
history-auto = { $control }: встановлено Авто
history-on = Увімк.
history-off = Вимк.
history-points =
    { $count ->
        [one] { $count } точка
        [few] { $count } точки
        [many] { $count } точок
       *[other] { $count } точки
    }
history-peek = Утримуйте, щоб побачити знімок без інструмента: { $tool }
history-peek-tip = Утримуйте, щоб побачити знімок без інструмента: { $tool }. Нічого не зберігається

## Крива

curve-point = Точка { $index }: вхід { $input }%, вихід { $output }%
curve-point-value = Вхід { $input }%, вихід { $output }%
curve-black-point = { $control }: точка чорного
curve-white-point = { $control }: точка білого
curve-keys = Page Up і Page Down вибирають точку, стрілки переміщують її, Insert або + додає точку, Delete видаляє її, а Shift+Delete — усі.
curve-keys-mac = Page Up і Page Down (Fn+↑ і Fn+↓) вибирають точку, стрілки переміщують її, + додає точку, Delete видаляє її, а Shift+Delete — усі.

## Графік смуг еквалайзера тонів; $tool — назва інструмента

equalizer-name = { $tool }, крива
equalizer-tip = Двічі клацніть, щоб скинути криву.
equalizer-keys = Page Up і Page Down вибирають смугу, стрілки переміщують її, Delete скидає її, а Shift+Delete — усі.
equalizer-keys-mac = Page Up і Page Down (Fn+↑ і Fn+↓) вибирають смугу, стрілки переміщують її, Delete скидає її, а Shift+Delete — усі.
equalizer-overflows-shadows = Тіні маски виходять за смугу −8 EV.
equalizer-overflows-highlights = Світлі тони маски виходять за смугу +0 EV.
equalizer-overflows-both = Тіні маски виходять за смугу −8 EV, а світлі тони — за смугу +0 EV.

## Гістограма й інші графіки; $shadows і $highlights — частки пікселів знімка, у відсотках

section-scopes = Графіки
    .search = Гістограми
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

## Піпетки (docs/pickers.md, The picker button). $tip — власні слова піпетки, $command —
## модифікатор вказівника, як його пишуть скорочення: «Cmd» на Mac, «Ctrl» деінде

picker-tip = Узяти колір зі знімка
picker-tip-area =
    { $tip }
    { $command }+клацання або клацання правою кнопкою, щоб вибрати область
picker-tip-area-quad =
    { $tip }
    { $command }+клацання, щоб вибрати область

## Активна піпетка на знімку (docs/pickers.md, On the photo, Accessibility). $tool — назва
## інструмента, $picker — назва піпетки: мітка її керування на квадраті повзунка, інакше власна;
## $command — модифікатор вказівника, як його пишуть скорочення, $what — власне речення
## піпетки

picker-bar-name = { $tool } · { $picker }
picker-area-label = { $tool } · { $picker }
picker-bar-area = Клацніть, щоб узяти малу область, або перетягніть її. Перетягніть кут, щоб змінити розмір. Клацання правою кнопкою скидає область. Esc завершує.
picker-bar-point = { $what } Esc завершує.
picker-bar-both-area = Перетягніть область на знімку. { $command }+клацання перемикає на точку. Клацання правою кнопкою скидає область. Esc завершує.
picker-bar-both-point = { $what } { $command }+клацання перемикає на область. Esc завершує.
picker-photo-area = Область піпетки, { $picker }
picker-photo-point = Точка піпетки, { $picker }
picker-photo-at-point = точка на { $x }%, { $y }%
picker-photo-at-area = область від { $left }%, { $top }% до { $right }%, { $bottom }%
picker-photo-up = Посунути вгору
picker-photo-down = Посунути вниз
picker-said-control = { $control } { $value }
picker-said-rgb = середнє RGB { $red }, { $green }, { $blue }

picker-turned-on = { $picker }: увімкнено
picker-turned-off = { $picker }: вимкнено
picker-turned-off-why = { $picker }: вимкнено, { $why }
picker-turned-replaced = { $before }: вимкнено, увімкнено іншу піпетку. { $picker }: увімкнено
picker-off-photo = інше фото
picker-off-closed = фото закрито
picker-off-control = змінено { $control }
picker-off-tool = змінено { $tool }
picker-off-tool-off = { $tool } вимкнено
picker-off-section = { $tool } згорнуто
picker-off-history = змінено історію
picker-off-reset = редагування скинуто
picker-off-reference = еталон очищено

## Колодязь курсорів (docs/cursors.md, The well).
cursor-well = Курсори
cursor-name-key = { $name }, { $key }
cursor-tip = { $name } · { $key } — { $how }
cursor-tip-how = { $name } — { $how }
cursor-tip-key = { $name } · { $key }
cursor-list =
    { $has ->
        [cursors] Курсори: { $group }
        [settings] Налаштування: { $group }
       *[both] Курсори й налаштування: { $group }
    }
cursor-select = Вибір
cursor-select-how = Клацніть позначку на фото, щоб вибрати її, перетягніть, щоб посунути
cursor-pan = Зсув
cursor-pan-how = Перетягніть, щоб зсунути збільшене фото, навіть над позначками; або утримуйте A з будь-яким курсором
cursor-group-select = Вибір
cursor-group-pan = Зсув
cursor-group-pick = Піпетки
cursor-well-no-photo = Відкрийте фото, щоб вибрати курсор

## Плашки піпетки (docs/pickers.md, Declaring pickers).
picker-patch-input = Вхід
picker-patch-input-tip = Вхідний колір, який має відобразитися на ціль.
picker-patch-target = Ціль
picker-patch-lch-tip = Ці координати LCh обчислено з координат CIE Lab 1976.
picker-patch-lightness = L: { $value }%
picker-patch-none = L: н/д
picker-patch-hue = h: { $value } °
picker-patch-hue-none = h: н/д
picker-patch-chroma = c: { $value }
picker-patch-chroma-none = c: н/д

reference-value = L { $lightness } %
reference-photo = · { $photo }
reference-this-photo = · цей знімок
reference-none = немає
reference-measuring = вимірювання…
reference-gone = Знімка, на якому його виміряно, більше немає
reference-more = Параметри еталона
reference-clear = Очистити еталон
reference-set = Задати еталон за ділянкою

last-pick-label = Останнє взяття
last-pick-value = ■ L { $before } % → ■ L { $after } % · { $change }
last-pick-promise = Взята ділянка до і після та експозиція, якої це потребувало
pick-warning-clipped = Ділянка має обрізані пікселі
pick-warning-clipped-tip = Її світлість занижена: виберіть ділянку без обрізаних світлих тонів
pick-warning-short = { $control } { $value }: ділянка має L { $reached } %, а не { $target } %
pick-warning-short-tip = { $control } досягла межі { $value }

## Інструмент експорту (docs/gui.md, The Export tool): модуль експорту darktable, назви
## з великої літери лише першого слова. $count — кількість знімків, один знімок — точне
## число 1. $file — назва знімка чи файлу, $source — вихідний raw, $folder — тека, $width
## і $height — пікселі, $problem — що зупиняє експорт.

export-button = Експортувати { photo-count }
export-reset = Скинути налаштування експорту
export-presets = Шаблони експорту
export-preset-default = Типові налаштування
export-metadata-note = Ця версія ще не записує у файли EXIF, місце зйомки й авторські права.
export-path-tip-unexact = { $source } → { $pattern }, буде заповнено під час експорту

## Підсумок, попередження, кнопка і рядок під нею

export-summary-title = Підсумок
export-summary-label-preset = Шаблон
export-summary-label-file-name = Назва файлу
export-summary-label-size = Розмір
export-summary-label-scale = Масштаб
export-summary-label-profile = Профіль
export-summary-label-format = Формат
export-summary-label-destination = Призначення
export-summary-size = { $width } × { $height } px
export-summary-size-print = { $width } × { $height } { $unit } ({ $pixels_width } × { $pixels_height } px)
export-summary-scale = { $percent }%
export-summary-jpeg = JPEG, якість { $quality }
export-summary-png = PNG, { $bits } біт

export-view-only-left-out =
    { $count ->
        [one] { $count } знімок лише для перегляду пропущено
        [few] { $count } знімки лише для перегляду пропущено
        [many] { $count } знімків лише для перегляду пропущено
       *[other] { $count } знімка лише для перегляду пропущено
    }
export-warn-replaces =
    { $count ->
        [one] Замінить { $count } файл, що вже є
        [few] Замінить { $count } файли, що вже є
        [many] Замінить { $count } файлів, що вже є
       *[other] Замінить { $count } файла, що вже є
    }
export-warn-replaces-at-least =
    { $count ->
        [one] Замінить щонайменше { $count } файл, що вже є
        [few] Замінить щонайменше { $count } файли, що вже є
        [many] Замінить щонайменше { $count } файлів, що вже є
       *[other] Замінить щонайменше { $count } файла, що вже є
    }
export-show-jobs = Показати завдання
export-failed = Експорт не вдався: { $done } з { $total }

# Підказка рядка чи групи, показаних вимкненими, бо їх ще не зроблено.
export-later = З'явиться в одній із наступних версій

## Параметри сховища

export-storage-options = Параметри сховища
export-target-storage = Цільове сховище
export-storage-latex = Шаблон книги LaTeX
export-storage-disk = Файл на диску
export-storage-email = Надіслати поштою
export-storage-gallery = Вебгалерея
export-path = Шлях
export-choose-folder = Вибрати теку
export-choose-folder-title = Виберіть теку
export-choose-folder-accept = Вибрати як теку для експорту
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
export-why-view-only-newer = Ці зміни записала новіша версія Apertrust.
export-why-view-only-unreadable = Історію в { $file } не вдається прочитати.

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
export-size-capped = { $file } матиме { $width } × { $height } px (без збільшення: «Дозволити збільшення» вимкнено).
export-size-full = { $file } матиме { $width } × { $height } px (повний розмір).
export-size-no-limit = { $file } матиме { $width } × { $height } px (повний розмір: межі не задано).
export-upscale = Дозволити збільшення
export-high-quality = Передискретизація високої якості
export-high-quality-tip = Передискретизація високої якості під час експорту: усі операції в повному розмірі, потім зменшення.
export-masks = Зберігати маски
export-masks-later = Зберігати маски шарами в експортованих зображеннях. Це вміють лише TIFF, OpenEXR і XCF, а шари з'являться в наступній версії.
export-option-no = Ні
export-option-yes = Так
export-profile = Профіль
export-profile-image = Налаштування зображення
export-profile-tip = Налаштування зображення: власний профіль знімка, у «Raw › Вихідний колірний профіль», як у перегляді. Інший профіль діє лише для цього експорту.
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
alert-refused-no-photo = Фото цієї обробки, { $photo }, немає в тій самій теці.
alert-refused-button = Гаразд
alert-running-title = Apertrust уже працює з цією текою
alert-running-message = Щоб запустити його з { $flags }, спершу закрийте Apertrust, а тоді запустіть знову.
alert-running-button = Гаразд
alert-discard-title = Відкинути історію цього знімка?
alert-discard-message = Знімок повернеться до того, яким він відкрився вперше, а його історію буде очищено. Її можна повернути командою Редагування › Скасувати, доки знімок відкритий.
alert-discard-unreadable-message = Частину історії цієї обробки не вдається прочитати. Відкидання почне її наново від знімка, яким він є зараз, а те, що не вдається прочитати, буде втрачено назавжди. Щоб зберегти це, спершу скопіюйте { $file }.
alert-discard-button = Відкинути історію
alert-preset-update-title = Оновити шаблон «{ $name }» поточними значеннями інструмента?
alert-preset-update-message = Його файл буде замінено.
alert-preset-update-button = Оновити
alert-preset-delete-title = Видалити шаблон «{ $name }»?
alert-preset-delete-message = Його файл буде вилучено.
alert-preset-delete-button = Видалити
alert-style-delete-title = Видалити стиль «{ $name }»?
alert-style-delete-message = Його файл буде видалено. Знімки, до яких його застосовано, збережуть свої редагування.
