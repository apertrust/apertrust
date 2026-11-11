# Interfejs Apertrust po polsku (docs/gui.md, Translation).
# Tłumaczenie komunikatów z en-GB/ui.ftl z tymi samymi zmiennymi. Wersja robocza: jeszcze
# niesprawdzona przez rodzimego użytkownika języka.

app-name = Apertrust

## Wspólne działania na górnym pasku

action-import = Importuj…
action-export = Eksportuj…
action-export-again = Eksportuj ponownie
action-reset = Resetuj
action-undo = Cofnij
action-redo = Ponów
action-copy-edit = Kopiuj edycję
action-apply-edit = Zastosuj edycję
action-copy-apply-parts = Kopiuj lub zastosuj część
action-settings = Ustawienia
status-saved = Zapisano

## Karty panelu bocznego

tab-photos = Zdjęcia
tab-base = Podstawowe
tab-color = Kolor
tab-correct = Korekcja
tab-effect = Efekt
tab-history = Historia
tab-metadata = Metadane
panel-tab-later = { $tab }: pojawi się w jednej z kolejnych wersji
panel-tabs = Karty panelu bocznego
panel-more-tabs = Więcej kart
panel-tab-options = Opcje kart
panel-show-tab-names = Pokaż nazwy kart
panel-customise-tabs = Dostosuj karty…
panel-width = Szerokość panelu bocznego

## Przyciski w nagłówku sekcji; $section to tytuł sekcji

section-reset = Resetuj: { $section }
section-presets = Ustawienia wstępne: { $section }
section-options = Opcje: { $section }
section-toggle = Otwórz lub zamknij: { $section }. Przytrzymaj H nad sekcją, aby zobaczyć zdjęcie bez niej.
section-tabs = Karty: { $section }
section-tab-later = Pojawi się w jednej z kolejnych wersji
choice-option-later = Pojawi się w jednej z kolejnych wersji
choice-option-unavailable = Niedostępne dla tego zdjęcia

## Zwijana grupa w module, zwijana sekcja darktable

collapsible-group-set = Zmieniono: { $count }
collapsible-group-tip = { $group }: zmieniono { $count }

## Wiersz ustawień wstępnych w module, jak Tryb balansu bieli

preset-custom = Własne
preset-option-unavailable = Niedostępne dla tego zdjęcia

## Znacznik suwaka

slider-kept = Zablokowany: ta wartość została ustawiona przez Ciebie, więc Auto i Dopasowanie jej nie zmieniają. Kliknij, aby odblokować.
slider-keep = Kliknij, aby zablokować tę wartość.
marker-keep = Zablokuj: { $control }
field-value = { $control }: dokładna wartość
field-value-in = { $control }, w { $unit }
slider-before-match = Przed Dopasowaniem: { $value }
slider-value-tip = Kliknij lub naciśnij Enter na suwaku, aby wpisać wartość.
slider-value-tip-mac = Kliknij lub naciśnij Return na suwaku, aby wpisać wartość.

## Motywy

theme-dark = Ciemny
theme-light = Jasny
theme-high-contrast = Wysoki kontrast

## Liczności

photo-count =
    { $count ->
        [one] { $count } zdjęcie
        [few] { $count } zdjęcia
        [many] { $count } zdjęć
       *[other] { $count } zdjęcia
    }

## Jednostki, po wartości

unit-ev = EV
unit-percent = %
unit-degrees = °
unit-kelvin = K
unit-pixels = px
unit-iso = ISO
unit-mib = MiB
unit-gb = GB

## Wartość z jednostką, jak pokazuje ją suwak i słyszą czytniki ekranu; $value jest już
## w formacie liczb języka

unit-ev-value = { $value } EV
unit-percent-value = { $value }%
unit-degrees-value = { $value }°
unit-kelvin-value = { $value } K
unit-pixels-value = { $value } px
unit-iso-value = { $value } ISO

## Panel górny

top-panel = Panel górny
top-panel-menu = Menu
top-panel-more = Więcej działań
status-saving = Zapisywanie…
status-saving-tip = Edycja tego zdjęcia wciąż jest zapisywana obok niego
status-not-saved = Nie zapisano
status-not-saved-reason = Nie zapisano: { $error }
status-view-only = Tylko podgląd
window-minimise = Minimalizuj
window-maximise = Maksymalizuj
window-restore = Przywróć
window-close = Zamknij

## Zadania: wskaźnik na końcu górnego paska i okno, które otwiera

jobs-name = Zadania
action-show-jobs = Zadania
jobs-tip = Zadania: { $value }
jobs-value-running = Uruchomione: { $running }
jobs-value-waiting = Oczekujące: { $waiting }
jobs-value-running-waiting = Uruchomione: { $running }, oczekujące: { $waiting }
jobs-value-failed = Błędy: { $failed }
jobs-empty = Brak zadań.
jobs-queued = Czeka na inne zadania
jobs-finished = Zakończono: { $message }
jobs-failed = Niepowodzenie: { $message }
jobs-cancelled = Anulowano: { $message }
jobs-cancel = Anuluj
jobs-cancel-tip = Zatrzymaj to zadanie
jobs-clear-finished = Wyczyść zakończone
jobs-live-started = Rozpoczęto: { $message }

## Komunikaty samych zadań

job-import = Importowanie { $done } / { $total }
job-delete = Usuwanie { $done } / { $total }
job-thumbnails = Tworzenie miniatur
job-folder = Odczytywanie { $folder }

## Czyszczenie pamięci podręcznej: $kind to, co jest czyszczone, thumbnails lub pipelines;
## $done to usunięte wpisy z $total; $entries to wpisy usunięte, $size i $unit, ile
## zajmowały, jednostka kb, mb lub gb.

job-cache-size =
    { $size } { $unit ->
        [kb] kB
        [gb] GB
       *[mb] MB
    }
job-cache-entries =
    { $kind ->
        [thumbnails]
            { $entries ->
                [one] { $entries } miniaturę
                [few] { $entries } miniatury
                [many] { $entries } miniatur
               *[other] { $entries } miniatury
            }
       *[pipelines]
            { $entries ->
                [one] { $entries } potok
                [few] { $entries } potoki
                [many] { $entries } potoków
               *[other] { $entries } potoku
            }
    }
job-cache-clear =
    { $kind ->
        [thumbnails] Czyszczenie pamięci podręcznej miniatur
       *[pipelines] Czyszczenie pamięci podręcznej potoków
    }
job-cache-clear-now = Usunięto { $done } z { $total }
job-cache-cleared = Usunięto { job-cache-entries }, { job-cache-size }
job-cache-clear-cancelled = Czyszczenie anulowane. Usunięto { job-cache-entries }, { job-cache-size }

job-export-photo =
    Eksportowanie { $number } / { $total } do { $storage ->
       *[disk] pliku na dysku
    }
job-export-queued = Następne w kolejce: { photo-count }, do folderu { $folder }
job-export-now =
    { $total ->
        [1] Eksportowanie: { $photo }
       *[other] Gotowe: { $done } z { $total } · teraz { $photo }
    }
job-export-between = Gotowe: { $done } z { $total }
job-export-finished =
    { $skipped ->
        [0]
            { $written ->
                [1] Wyeksportowano 1 zdjęcie
                [few] Wyeksportowano { $written } zdjęcia
                [many] Wyeksportowano { $written } zdjęć
               *[other] Wyeksportowano { $written } zdjęcia
            }
       *[other]
            { $total ->
                [1] Zdjęcie pominięto
               *[other] Wyeksportowano { $written } z { $total } zdjęć, pominięto: { $skipped }
            }
    }
job-export-failed-outcome =
    { $total ->
        [1] Nie udało się wyeksportować zdjęcia
       *[other] Eksport nieudany. Wyeksportowano { $written } z { $total } zdjęć
    }
job-export-cancelled =
    { $total ->
        [1] Eksport anulowany
       *[other] Eksport anulowany. Wyeksportowano { $written } z { $total } zdjęć
    }

job-cause-unreadable = nie można odczytać pliku raw
job-cause-edit = nie można odczytać jego edycji
job-cause-binding = zmienionego modułu nie można zastosować do zdjęcia
job-cause-size = żądany rozmiar nie pasuje do formatu
job-cause-pattern = nazwa pliku jest nieprawidłowa
job-cause-name = nazwa pliku nie daje temu zdjęciu nazwy
job-cause-render = nie udało się przetworzyć zdjęcia
job-cause-encode = nie udało się zakodować pliku
job-cause-disk-full = dysk jest pełny
job-cause-read-only = dysk lub folder jest tylko do odczytu
job-cause-gone = folderu lub jego dysku już nie ma
job-cause-folder = nie można utworzyć folderu ani zapisać w nim
job-cause-write = nie udało się zapisać pliku
job-cause-engine = silnik nieoczekiwanie się zatrzymał
job-cause-cache-remove = nie udało się usunąć niektórych plików pamięci podręcznej
job-cause-cache-read = nie można odczytać folderu pamięci podręcznej

## Wiersz zadania w oknie zadań: wiersze i przyciski

activity-folder = Folder: { $folder }
activity-folder-at = Folder: { $folder } · { $time }
activity-failure = { $photo }: nie udało się zapisać { $file }: { $cause }.
activity-failure-photo = { $photo }: { $cause }.
activity-failure-cause = { $cause }.
activity-show-finder = Pokaż w Finderze
activity-show-explorer = Pokaż w Eksploratorze
activity-show-files = Pokaż w Plikach
activity-show-tip = Otwórz folder eksportu
activity-try-again = Ponów
activity-try-again-tip = Wyeksportuj ponownie zdjęcia, które się nie powiodły, z tymi samymi ustawieniami
activity-dismiss = Zamknij

## Pasek centralny, nad zdjęciem

central-bar = Pasek centralny
photo-show-selected = Pokaż wszystkie zaznaczone zdjęcia
photo-proof-margin = Margines próbny
photo-before-after = Przed i po
photo-guides = Linie pomocnicze
guides-show-label = Pokaż prowadnice
guides-show-full = Pokaż prowadnice
guides-show-promise = Pokaż nakładkę prowadnic, kiedy moduł jest aktywny.
guides-settings-full = Zmień globalne ustawienia prowadnic
guides-settings-promise = Te ustawienia są globalne i dotyczą każdego modułu, w którym występują nakładki prowadnic.
photo-layer = Warstwa: { $layer }. Zmień warstwę
photo-new-layer = Nowa warstwa
photo-values = Wartości pod kursorem
photo-values-read = Czerwony { $red }, zielony { $green }, niebieski { $blue }, szary { $grey }
photo-zoom = Powiększenie
photo-zoom-small = Mały
photo-zoom-fit = Dopasuj
photo-zoom-fill = Wypełnij
photo-zoom-in = Powiększ
photo-zoom-out = Pomniejsz
photo-zoom-tip = Powiększenie: uszczypnięcie lub { $command }+przewijanie; Z – 100%. Przewijanie przesuwa powiększone zdjęcie, Shift+przewijanie – w bok
photo-zoom-tip-exact = Przy 100% ekran pokazuje dokładnie to, czym będzie eksport. Powiększenie: uszczypnięcie lub { $command }+przewijanie; Z – 100%. Przewijanie przesuwa powiększone zdjęcie, Shift+przewijanie – w bok
photo-zoom-later = Powiększenie pojawi się wraz z widokiem 100%
layer-image = Obraz

## Panel centralny

photo-opening = Otwieranie { $name }…
photo-rendering = Renderowanie…
controls-opening = Otwieranie { $name }…
edit-not-saved = Edycji { $name } nie zapisano: { $error }
photo-view-only = Edytowane w nowszym Apertrust, { $version }: otwarte tylko do podglądu
photo-view-only-unknown = Edytowane w nowszym Apertrust: otwarte tylko do podglądu
photo-shutter = { $value } s
photo-aperture = f/{ $value }
photo-focal-length = { $value } mm
photo-iso = ISO { $value }

## Okno otwierania, z menu Plik › Otwórz…

open-title = Otwórz
open-filter = Zdjęcia raw

## Panel siatki i jego komórki

grid-panel = Panel siatki
grid-cell-edited = Edytowane
grid-cell-view-only = Tylko podgląd: edycję zapisała nowsza wersja Apertrust
grid-cell-unreadable = Nie udało się odczytać edycji: { $reason }
grid-cell-rating =
    { $count ->
        [0] Bez gwiazdek
        [one] { $count } gwiazdka
        [few] { $count } gwiazdki
        [many] { $count } gwiazdek
       *[other] { $count } gwiazdki
    }
grid-cell-rejected = Odrzucone
grid-cell-labels = Etykiety kolorów: { $labels }
grid-cell-rate =
    { $count ->
        [one] Oceń na { $count } gwiazdkę
        [few] Oceń na { $count } gwiazdki
        [many] Oceń na { $count } gwiazdek
       *[other] Oceń na { $count } gwiazdki
    }
grid-cell-reject = Odrzuć
grid-cell-colour-labels = Etykiety kolorów
grid-cell-no-labels = Brak
grid-cell-clear-labels = Usuń etykiety
colour-label-red = Czerwony
colour-label-yellow = Żółty
colour-label-green = Zielony
colour-label-blue = Niebieski
colour-label-purple = Fioletowy
grid-panel-height = Wysokość panelu siatki
grid-rows =
    { $count ->
        [one] { $count } wiersz
        [few] { $count } wiersze
        [many] { $count } wierszy
       *[other] { $count } wiersza
    }
grid-panel-width = Szerokość panelu siatki
grid-columns =
    { $count ->
        [one] { $count } kolumna
        [few] { $count } kolumny
        [many] { $count } kolumn
       *[other] { $count } kolumny
    }
grid-bar-options = Opcje panelu siatki
grid-position = Położenie
grid-bar = Pasek siatki
grid-sort = Sortowanie
grid-sort-value =
    { $key ->
        [capture-time] Data wykonania
        [file-name] Nazwa pliku
        [edited] Edytowane
       *[rating] Ocena
    }, { $descending ->
        [yes] malejąco
       *[no] rosnąco
    }
grid-sort-tip =
    Sortowanie: { $key ->
        [capture-time] data wykonania
        [file-name] nazwa pliku
        [edited] edytowane
       *[rating] ocena
    }, { $descending ->
        [yes] malejąco
       *[no] rosnąco
    }
grid-thumbnail-size = Rozmiar miniatur
grid-thumbnail-size-value =
    { $size ->
        [small] Małe
        [medium] Średnie
        [large] Duże
       *[extra-large] Bardzo duże
    }
grid-thumbnail-size-tip =
    Rozmiar miniatur: { $size ->
        [small] małe
        [medium] średnie
        [large] duże
       *[extra-large] bardzo duże
    }
grid-labels = Podpisy

## Co jest otwarte, według rodzaju

open-kind-folder = Folder: { $name }

## Tryb folderu: Plik › Otwórz folder…, sekcja Foldery karty Biblioteka i puste stany

action-open-folder = Otwórz folder…
open-folder-title = Otwórz folder
section-browse = Przeglądaj
browse-folders = Foldery
browse-open-name = Co jest otwarte
browse-no-folder = Nie otwarto folderu
browse-recents-none = Brak ostatnich folderów
browse-recent-missing = { $label }, nie znaleziono
browse-try-again = Spróbuj ponownie
browse-locate = Wskaż…
browse-remove = Usuń z listy
browse-locate-title = Wskaż folder
browse-plus = Nowy lub otwórz
action-new-library = Nowa biblioteka…
action-new-project = Nowy projekt…
browse-folder-menu = Menu folderu { $name }
browse-open-row = Otwórz
browse-open-this-folder = Otwórz ten folder
browse-reading = Odczytywanie…
browse-unreadable = Nie udało się odczytać. Spróbuj ponownie
folders-reading = Odczytywanie { $name }…
folders-recent-missing = Nie udało się otworzyć tego folderu. { $reason }
folders-recent-gone = Tego folderu już tam nie ma.
folder-reading = Odczytywanie { $folder }…
folder-empty = Brak dostępnych zdjęć
folder-empty-hint = Wybierz podfolder w sekcji „Przeglądaj”.
folder-failed = Nie udało się otworzyć { $folder }: { $reason }
folder-cancelled = Odczytywanie { $folder } przerwano
folder-no-photo = Nie wybrano zdjęcia

## Okno startowe, wyświetlane zamiast głównego okna, gdy nic nie jest otwarte

start-recents = Ostatnie
start-recents-none = Jeszcze nic nie otwarto
start-kind-folder = Folder
start-recent-gone = Nie znaleziono
start-recent-failed = Nie udało się otworzyć
start-recent-about = { $kind }, { $path }
start-recent-about-missing = { $kind }, { $path }. { $why }
start-browse = Przeglądaj…
start-open = Otwórz
start-copyright = © 2026
start-tagline =
    Wolny, nieniszczący
    edytor zdjęć raw

## Ustawienia

settings-close = Zamknij
settings-overridden = Ustawione przez { $flag } na to uruchomienie
settings-after-restart = Zacznie działać po ponownym uruchomieniu
settings-changed = Zmieniono względem wartości domyślnej
settings-reset = Resetuj
settings-page-general = Ogólne
settings-page-import = Import
settings-page-photos = Zdjęcia
settings-page-editing = Edycja
settings-page-processing = Przetwarzanie
settings-page-storage = Przechowywanie
settings-page-shortcuts = Skróty klawiszowe
settings-page-presets = Presety
settings-page-ai = SI
settings-page-later = Pojawi się w jednej z kolejnych wersji
settings-group-interface = Interfejs
settings-group-start = Uruchamianie
settings-group-confirmations = Potwierdzenia
settings-group-thumbnails = Miniatury
settings-group-side-panel = Panel boczny
settings-group-gpu = GPU
settings-group-diagnostics = Diagnostyka
settings-group-cache = Pamięć podręczna
setting-theme-label = Motyw
setting-theme-option-dark = Ciemny
setting-theme-option-light = Jasny
setting-theme-option-high-contrast = Wysoki kontrast
setting-locale-label = Język
setting-scale-label = Rozmiar interfejsu
setting-sliders-marker-label = Uchwyt suwaka
setting-sliders-marker-option-triangle = Trójkąt
setting-sliders-marker-option-circle = Koło
setting-sliders-marker-option-square = Kwadrat
setting-library-reopen-last-label = Otwieraj ponownie to, co było otwarte ostatnio
setting-library-reopen-last-help = Otwiera folder, w którym pracowano, gdy { app-name } został zamknięty, wraz z jego zdjęciem. Plik lub folder otwarty z menedżera plików lub terminala otwiera się zamiast niego.
setting-export-ask-before-overwrite-label = Pytaj przed nadpisaniem wyeksportowanego pliku
setting-grid-panel-square-thumbnails-label = Kwadratowe miniatury
setting-side-panel-tab-names-label = Pokaż nazwy kart
setting-engine-adapter-label = Karta graficzna
setting-engine-adapter-hint = Automatycznie
setting-engine-memory-budget-label = Budżet pamięci
setting-cache-limited-label = Ogranicz rozmiar pamięci podręcznej
setting-cache-limited-help = Gdy wyłączone, pamięć podręczna jest nieograniczona: nic z niej nie jest usuwane, by zmieściła się w rozmiarze.
setting-cache-limit-label = Limit pamięci podręcznej
setting-cache-limit-help = Powyżej tego rozmiaru usuwane są miniatury najdawniej używane; w razie potrzeby zostaną utworzone ponownie.
setting-cache-unused-after-label = Pamięci podręczne bibliotek nieużywane od
setting-cache-unused-after-help = Pamięć podręczna biblioteki nieotwierana na tym komputerze tak długo zostanie zaproponowana do usunięcia. Nic nie jest usuwane bez Ciebie.
setting-cache-unused-after-option-days_30 = 30 dni
setting-cache-unused-after-option-days_90 = 90 dni
setting-cache-unused-after-option-days_180 = 180 dni
setting-cache-unused-after-option-days_365 = 365 dni
setting-cache-unused-after-option-never = Nigdy
setting-log-keep-label = Prowadź dziennik tej sesji
setting-log-keep-help = Zachowuje dziennik tej sesji w pamięci od chwili włączenia, dla Pomoc › Pokaż dziennik. Aby prowadzić go od samego początku, uruchom Apertrust ponownie. Nic nie trafia do pliku, dopóki go nie zapiszesz.
setting-log-domains-label = Dziennik
setting-log-domains-help = Co dziennik zachowuje oprócz ostrzeżeń i błędów, które zachowuje zawsze. „Szczegółowo” dodaje znacznie więcej, dla programisty.
setting-log-domains-option-gpu = GPU
setting-log-domains-option-raw = Raw
setting-log-domains-option-pipe = Potok
setting-log-domains-option-perf = Wydajność
setting-log-domains-option-memory = Pamięć
setting-log-domains-option-settings = Ustawienia
setting-log-domains-option-control = Sterowanie
setting-log-domains-option-event = Zdarzenia
setting-log-domains-option-verbose = Szczegółowo

## Ustawienia › Przechowywanie, jego część zbudowana ręcznie (decyzja 76). $flag i $variable
## to to, co wskazało folder; $kind to thumbnails, pipelines lub libraries; $entries, $size
## i $unit jak w czyszczeniu pamięci podręcznej; $limit to limit w GB.

settings-group-folders = Foldery
settings-group-cache-contents = W pamięci podręcznej
settings-storage-user-data = Folder danych użytkownika
settings-storage-cache = Folder pamięci podręcznej
settings-storage-chosen-flag = Wskazany przez { $flag }
settings-storage-chosen-variable = Wskazany przez { $variable }
settings-storage-chosen-user-data = Wewnątrz folderu danych użytkownika
settings-storage-chosen-platform = Folder systemowy
settings-storage-none = Brak: system nie podaje folderu
settings-storage-show-tip = Otwórz ten folder
settings-storage-show-name = { $show }: { $folder }
settings-storage-kind-thumbnails = Miniatury
settings-storage-kind-pipelines = Potoki
settings-storage-kind-libraries = Pamięci podręczne bibliotek
settings-storage-entries =
    { $kind ->
        [thumbnails]
            { $entries ->
                [one] { $entries } miniatura
                [few] { $entries } miniatury
                [many] { $entries } miniatur
               *[other] { $entries } miniatury
            }
        [libraries]
            { $entries ->
                [one] { $entries } biblioteka
                [few] { $entries } biblioteki
                [many] { $entries } bibliotek
               *[other] { $entries } biblioteki
            }
       *[pipelines]
            { $entries ->
                [one] { $entries } potok
                [few] { $entries } potoki
                [many] { $entries } potoków
               *[other] { $entries } potoku
            }
    }
settings-storage-usage = { settings-storage-entries }, { job-cache-size }
settings-storage-total = Razem
settings-storage-total-limit = { job-cache-size } z { $limit } GB
settings-storage-total-unlimited = { job-cache-size }, bez limitu
settings-storage-counting = Liczenie…
settings-storage-clear = Wyczyść
settings-storage-clear-thumbnails = Wyczyść miniatury
settings-storage-clear-pipelines = Wyczyść potoki
settings-storage-clear-tip = Usuwane z dysku i tworzone ponownie w razie potrzeby
settings-storage-no-cache = Brak pamięci podręcznej na dysku: przy uruchomieniu Apertrust nie udało się zapisać jej folderu, więc nic nie jest zachowywane między uruchomieniami.
settings-storage-refused-clearing = Są już czyszczone.
settings-storage-refused-no-cache = Nie ma pamięci podręcznej do wyczyszczenia.
settings-storage-refused = Nie udało się ich wyczyścić.

## Okno dziennika, Pomoc › Pokaż dziennik; jego filtry to opcje wiersza „Dziennik”

action-show-log = Pokaż dziennik
log-name = Dziennik
log-off-title = Dziennik nie jest prowadzony
log-off-text = Włącz go w ustawieniach, a od tej chwili wiersze będą zachowywane w pamięci, tylko dla tej sesji.
log-open-settings = Otwórz ustawienia diagnostyki…
log-filters = Pokaż
log-filter-warnings = Ostrzeżenia i błędy
log-filter-not-kept = Niezachowywane: włącz w Ustawienia › Ogólne › Diagnostyka
log-lines = Wiersze
log-dropped = Najstarszych wierszy już nie ma: { $count }. Dziennik zachowuje 20 000 najnowszych.
log-newer = ↓ Nowsze wiersze: { $count }
log-counts = Zachowane wiersze: { $kept } · pokazane: { $shown } · tylko ta sesja, w pamięci
log-names-files = Wiersze zawierają nazwy Twoich plików i folderów.
log-save = Zapisz do pliku…
log-copy = Kopiuj do schowka
log-save-title = Zapisz dziennik

## O programie Apertrust

action-about = O programie { app-name }
about-version = Wersja { $version }
about-copyright = Prawa autorskie © 2026 twórcy { app-name }.
about-free-software = Wolne oprogramowanie na licencji GNU General Public License w wersji 3 lub późniejszej.
about-credits = Podziękowania
about-licence = Licencja
about-back = Wstecz

## Działania, jak nazywa je mapa klawiszy

action-open = Otwórz…
action-view-edit = Przejdź do edycji
action-show-browse = Przeglądaj
action-panel-tab-1 = Karta panelu bocznego 1
action-panel-tab-2 = Karta panelu bocznego 2
action-panel-tab-3 = Karta panelu bocznego 3
action-panel-tab-4 = Karta panelu bocznego 4
action-panel-tab-5 = Karta panelu bocznego 5
action-scale-up = Powiększ interfejs
action-scale-down = Pomniejsz interfejs
action-scale-reset = Normalny rozmiar interfejsu
action-next-region = Następny panel
action-previous-region = Poprzedni panel
action-full-screen = Pełny ekran
action-close-window = Zamknij okno
action-close = Zamknij
action-open-recent-1 = Otwórz ostatnie 1
action-open-recent-2 = Otwórz ostatnie 2
action-open-recent-3 = Otwórz ostatnie 3
action-open-recent-4 = Otwórz ostatnie 4
action-open-recent-5 = Otwórz ostatnie 5
action-open-recent-6 = Otwórz ostatnie 6
action-open-recent-7 = Otwórz ostatnie 7
action-open-recent-8 = Otwórz ostatnie 8
action-open-recent-9 = Otwórz ostatnie 9
action-open-recent-10 = Otwórz ostatnie 10
action-clear-recent = Wyczyść menu
action-quit = Zakończ
action-peek = Pokaż zdjęcie bez sekcji pod wskaźnikiem
action-next-photo = Następne zdjęcie
action-previous-photo = Poprzednie zdjęcie
action-zoom-in = Powiększ
action-zoom-out = Pomniejsz
action-zoom-fit = Dopasuj
action-zoom-100 = Powiększenie 100%
action-sort-capture-time = Data wykonania
action-sort-file-name = Nazwa pliku
action-sort-edited = Edytowane
action-sort-rating = Ocena
action-sort-ascending = Rosnąco
action-sort-descending = Malejąco
action-thumbnails-small = Małe
action-thumbnails-medium = Średnie
action-thumbnails-large = Duże
action-thumbnails-extra-large = Bardzo duże
action-grid-position-left = Po lewej
action-grid-position-right = Po prawej
action-grid-position-bottom = Na dole
action-show-labels = Pokazuj podpisy
action-background-black = Czarne
action-background-very-dark = Bardzo ciemne
action-background-dark = Ciemne
action-background-medium = Średnie
action-background-light = Jasne
action-background-white = Białe
key-space = Spacja

## Tytuły menu; menu aplikacji nazywa się app-name

-file-menu = Plik
menu-file = &{ -file-menu }
menu-edit = &Edycja
menu-photo = &Zdjęcie
menu-view = &Widok
menu-window = &Okno
# P is Plik's.
menu-help = Po&moc
menu-panel-tab = Karta panelu bocznego
menu-interface-size = Rozmiar interfejsu
menu-sort-by = Sortuj według
menu-thumbnail-size = Rozmiar miniatur
menu-background = Tło
menu-grid-position = Położenie panelu siatki
menu-open-recent = Otwórz ostatnie
menu-recent-in = { $name } — { $parent }
menu-services = Usługi
menu-hide-app = Ukryj { app-name }
menu-hide-others = Ukryj pozostałe
menu-show-all = Pokaż wszystko
menu-minimise = Minimalizuj
menu-zoom-window = Powiększ
menu-bring-all-to-front = Umieść wszystko na wierzchu

## Krzywa

curve-point = Punkt { $index }: wejście { $input }%, wyjście { $output }%
curve-point-value = Wejście { $input }%, wyjście { $output }%
curve-black-point = { $control }: punkt czerni
curve-white-point = { $control }: punkt bieli
curve-keys = Page Up i Page Down wybierają punkt, strzałki go przesuwają, Insert lub + dodaje punkt, Delete go usuwa, a Shift+Delete usuwa wszystkie.
curve-keys-mac = Page Up i Page Down (Fn+↑ i Fn+↓) wybierają punkt, strzałki go przesuwają, + dodaje punkt, Delete go usuwa, a Shift+Delete usuwa wszystkie.

## Wykres pasm korektora tonów; $module to nazwa modułu

equalizer-name = { $module }, krzywa
equalizer-tip = Kliknij dwukrotnie, aby zresetować krzywą.
equalizer-keys = Page Up i Page Down wybierają pasmo, strzałki je przesuwają, Delete je resetuje, a Shift+Delete resetuje wszystkie.
equalizer-keys-mac = Page Up i Page Down (Fn+↑ i Fn+↓) wybierają pasmo, strzałki je przesuwają, Delete je resetuje, a Shift+Delete resetuje wszystkie.
equalizer-overflows-shadows = Cienie maski wychodzą poza pasmo −8 EV.
equalizer-overflows-highlights = Światła maski wychodzą poza pasmo +0 EV.
equalizer-overflows-both = Cienie maski wychodzą poza pasmo −8 EV, a światła poza pasmo +0 EV.

## Histogram i inne wykresy; $shadows i $highlights to udział pikseli zdjęcia, w procentach

section-scopes = Wykresy
scope-histogram = Histogram
scope-waveform = Przebieg
scope-parade = Parada RGB
scope-vectorscope = Wektoroskop
scope-graphs = Wykres
scope-histogram-tip = Ile pikseli ma każdą jasność: czerń po lewej, biel po prawej.
scope-waveform-tip = Jasność w poprzek zdjęcia, od lewej do prawej jak na zdjęciu: ciemne na dole, jasne u góry.
scope-parade-tip = Przebiegi czerwieni, zieleni i błękitu obok siebie. Gdzie się pokrywają, zdjęcie jest neutralne.
scope-vectorscope-tip = Kolory według odcienia, czyli kąta, i nasycenia, czyli odległości od środka.
scope-log-scale = Skala logarytmiczna
scope-red = Kanał czerwony
scope-green = Kanał zielony
scope-blue = Kanał niebieski
scope-luminance = Luminancja
scope-show-shadow-clipping = Pokaż przycięte cienie
scope-show-highlight-clipping = Pokaż przycięte światła
scope-no-clipping = Bez przycięć
scope-clipped-shadows = Przycięte cienie: { $shadows }%
scope-clipped-highlights = Przycięte światła: { $highlights }%
scope-clipped-both = Przycięte cienie: { $shadows }%, światła: { $highlights }%
scope-empty = Nic jeszcze nie zmierzono

## Okno eksportu (docs/gui.md, The Export dialog): moduł eksportu darktable, nazwy
## wielką literą tylko pierwszego słowa. $count to liczba zdjęć, jedno zdjęcie to dokładnie
## liczba 1. $file to nazwa zdjęcia lub pliku, $source plik raw, $folder folder, $width
## i $height piksele, $problem to, co zatrzymuje eksport.

export-title = Eksport: { photo-count }
export-button = Eksportuj { photo-count }
export-close = Zamknij
export-hint = Eksport działa w tle; możesz dalej pracować.
# Podpowiedź wiersza lub grupy pokazanych jako wyłączone, bo jeszcze ich nie ma.
export-later = Pojawi się w jednej z kolejnych wersji

## Kolumna „Presety”

export-presets = Presety
export-preset-default = Ustawienia domyślne
export-preset-built-in = Wbudowany
export-preset-default-tip = Ustawienia takie, jak w darktable po instalacji: wybranie go je przywraca. Nie można go zmienić ani usunąć.
export-preset-matches-none = Ustawienia nie pasują do żadnego presetu.
export-presets-later = Własne presety pojawią się w jednej z kolejnych wersji.

## Opcje miejsca zapisu

export-storage-options = Opcje miejsca zapisu
export-target-storage = Miejsce docelowe
export-storage-latex = Szablon książki LaTeX
export-storage-disk = Plik na dysku
export-storage-email = Wyślij e-mailem
export-storage-gallery = Galeria internetowa
export-path = Ścieżka
export-choose-folder = Wybierz…
# Jak czytniki ekranu nazywają listę w polu ścieżki.
path-variables = Zmienne
export-on-conflict = W razie konfliktu
export-conflict-unique = Utwórz unikalną nazwę
export-conflict-overwrite = Nadpisz
export-conflict-changed = Nadpisz, jeśli zmieniono
export-conflict-skip = Pomiń
export-conflict-overwrite-no-alert = Nadpisywanie pyta przed zastąpieniem plików, a tego pytania nie da się tu jeszcze pokazać. Wybierz inny sposób.

## Co sesja mówi pod ścieżką i pod konfliktem

# $source to plik raw, $file to miejsce jego pliku.
export-name = { $source } → { $file }
export-names-more = i jeszcze { $count }, tak samo
export-no-name = Ścieżka nie daje nazwy pliku dla tego zdjęcia.
export-sequence-added = Zdjęć: { $count }, a w ścieżce nie ma zmiennej: dodawane jest _$(SEQUENCE), aby żaden plik nie nadpisał innego.
# $names to zmienne darktable, dla których Apertrust nie ma wartości.
export-not-here = { $names }: z darktable, tu bez wartości; rozwija się w nic.
export-conflict-one-number = { $file } już istnieje, więc ten plik zostanie zapisany z dodanym numerem.
export-conflict-one-overwrite = Zastąpi plik { $file }, który już istnieje.
export-conflict-one-skip = Pominie plik { $file }, który już istnieje.
export-conflict-one-unchanged = Pominie plik { $file }, który już istnieje i jest nowszy niż edycja zdjęcia.
export-conflict-many-number = Już istnieją: { $count } z { $total }; zostaną zapisane z dodanym _01.
export-conflict-many-overwrite = Zastąpi już istniejące: { $count } z { $total }.
export-conflict-many-skip = Pominie już istniejące: { $count } z { $total }.

## Co zatrzymuje eksport (D8)

export-problem-empty = Podaj, gdzie zapisać eksportowane zdjęcia.
export-problem-nameless = Po „$(” brakuje nazwy zmiennej. Wpisz $(, aby zobaczyć zmienne.
export-problem-unknown = Nieznana zmienna $({ $name }). Wpisz $(, aby zobaczyć zmienne.
export-problem-unknown-near = Nieznana zmienna $({ $name }). Czy chodziło o $({ $suggestion })?
export-fix = Użyj $({ $name })
export-problem-unclosed = Brakuje „{ $expected }” po „{ $variable }”.
export-problem-no-operation = „{ $found }” po { $variable } nie jest operacją.
export-problem-too-deep = Zmienne są zagnieżdżone głębiej, niż wolno: { $depth }.
export-problem-not-connected = Nie można zapisać w „{ $folder }”: dysk nie jest podłączony. Podłącz go lub wybierz inny folder.
export-problem-not-a-folder = Nie można zapisać w „{ $folder }”: w tym miejscu jest plik.
export-problem-read-only = Nie można zapisać w „{ $folder }”: tylko do odczytu.
export-problem-unreadable = Nie można zapisać w „{ $folder }”: { $why }.
export-problem-too-large = { $format }: co najwyżej { $largest } px z jednej strony.
export-problem-no-pixels = Rozmiar jest mniejszy niż piksel: { $width } × { $height } px.
export-problem-failed = Nie można wyeksportować zdjęcia w tym rozmiarze: { $why }
# Podpowiedź przycisku „Eksportuj”, gdy jest wyłączony; $problem to jeden z komunikatów wyżej.
export-why-path = Najpierw popraw ścieżkę: { $problem }
export-why-size = Najpierw popraw rozmiar: { $problem }

## Opcje formatu

export-format-options = Opcje formatu
export-file-format = Format pliku
export-format-copy = Kopia
export-jpeg-quality = Jakość
export-jpeg-subsample = Podpróbkowanie kolorów
export-jpeg-subsample-tip = Podpróbkowanie kolorów dla kodera JPEG. Auto: według jakości. 4:4:4: brak. 4:4:0: kolor zmniejszony o połowę w pionie. 4:2:2: w poziomie. 4:2:0: w obu kierunkach.
export-subsample-auto = Auto
export-png-bit-depth = Głębia bitowa
export-png-compression = Kompresja
export-bits-8 = 8 bitów
export-bits-16 = 16 bitów

## Opcje globalne

export-global-options = Opcje globalne
export-set-size = Ustaw rozmiar
export-mode-pixels = W pikselach (dla pliku)
export-mode-cm = W cm (do druku)
export-mode-inch = W calach (do druku)
export-mode-scale = Według skali (dla pliku)
export-size-pixels = Rozmiar w pikselach
export-size-cm = Rozmiar w centymetrach
export-size-inch = Rozmiar w calach
export-unit-cm = cm
export-unit-inch = cal
export-width = Szerokość
export-height = Wysokość
export-dpi = Rozdzielczość
export-dpi-unit = dpi
export-equal-to = Co równa się { $width } × { $height } px
export-scale = Skala
export-scale-tip = Liczba całkowita, ułamek dziesiętny lub prosty ułamek, na przykład 1/2. 0 lub puste to 1.
# Rozmiar aktywnego zdjęcia pod wierszami; $file to jego nazwa.
export-size = { $file } będzie miało { $width } × { $height } px.
export-size-capped = { $file } będzie miało { $width } × { $height } px (bez powiększania: „Zezwalaj na powiększanie” to Nie).
export-size-full = { $file } będzie miało { $width } × { $height } px (pełny rozmiar).
export-size-no-limit = { $file } będzie miało { $width } × { $height } px (pełny rozmiar: nie ustawiono limitu).
export-upscale = Zezwalaj na powiększanie
export-high-quality = Przepróbkowanie wysokiej jakości
export-high-quality-tip = Przepróbkowanie wysokiej jakości podczas eksportu: wszystkie operacje w pełnym rozmiarze, potem zmniejszenie.
export-no = Nie
export-yes = Tak
export-masks = Zapisuj maski
export-masks-later = Zapisuj maski jako warstwy w eksportowanych obrazach. Potrafią to tylko TIFF, OpenEXR i XCF, a warstwy pojawią się w kolejnej wersji.
export-profile = Profil
export-profile-image = Ustawienia obrazu
export-profile-tip = Ustawienia obrazu: własny profil zdjęcia, w „Kolor › Wyjściowy profil koloru”, tak jak w podglądzie. Inny profil dotyczy tylko tego eksportu.
export-intent = Zamiar
export-intent-perceptual = Percepcyjny
export-intent-relative = Względny kolorymetryczny
export-intent-saturation = Nasycenie
export-intent-absolute = Bezwzględny kolorymetryczny
export-intent-tip = Przy profilach macierzowych i bez plików ICC zamiar niczego nie zmienia, więc czeka na pliki ICC.
export-style = Styl
export-style-none = Brak
export-style-later = Style pojawią się w jednej z kolejnych wersji.
export-metadata = Metadane
export-multi-preset = Eksport według kilku presetów

## Alerty (docs/gui.md, Dialogs and alerts): pyta o nie sam system. $stoppable to liczba
## trwających zadań, które zakończenie zatrzymuje, $waiting -- na ile czeka.

alert-cancel = Anuluj
alert-overwrite-title = Wyeksportować z nadpisywaniem?
alert-overwrite-message = Eksport zastąpi istniejące pliki o tych samych nazwach. Starych plików nie będzie można przywrócić.
alert-overwrite-button = Nadpisz
alert-overwrite-suppress = Nie pytaj ponownie
alert-quit-title = Zakończyć, gdy trwa zadanie?
alert-quit-message =
    { $stoppable ->
        [0] Apertrust zakończy działanie, gdy trwające zadania się skończą.
       *[other]
            { $waiting ->
                [0] Trwające zadania zatrzymają się po zdjęciu, które jest zapisywane, i Apertrust zakończy działanie. Zapisane już pliki pozostaną.
               *[other] Zadania, które można zatrzymać, zatrzymają się po zdjęciu, które jest zapisywane, a Apertrust zakończy działanie, gdy skończą się pozostałe. Zapisane już pliki pozostaną.
            }
    }
alert-quit-button = Zakończ
alert-save-title = Edycja nie jest zapisana
alert-save-message = Nie udało się zapisać edycji { $name } obok zdjęcia: { $error }
alert-save-button = Spróbuj ponownie
alert-quit-save-title = Zakończyć bez ostatnich zmian?
alert-quit-save-message = Nie udało się zapisać ostatnich zmian: { $error }
alert-quit-anyway = Zakończ mimo to
alert-refused-title = Nie można otworzyć { $name }
alert-refused-unknown = Tu otwierają się tylko foldery i zdjęcia raw.
alert-refused-library = Biblioteki i projekty pojawią się w jednej z kolejnych wersji.
alert-refused-button = OK
