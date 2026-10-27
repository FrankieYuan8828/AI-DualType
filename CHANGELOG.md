# Changelog

> **MC-TypeToAnything**: A bidirectional IME that makes learning English easy, and makes learning/typing Chinese effortless for foreigners.

All notable changes to this project will be documented in this file.

## [v1.0.3] - 2026-10-27

### Added & Improved
- **1-Click MSI Installer**: Added standard Windows `.msi` installer for effortless installation by non-technical users.
- **Project Renamed**: Officially updated project name to **MC-TypeToAnything**.
- **Chinese Syllable Capacity & Paging Fixes**:
  - Fixed hard limit of 10 single characters per syllable in `build_dict.py`, increasing capacity to 100 characters per syllable.
  - Resolved missing character `浮` under `fu` (promoted to Candidate #3 on Page 1) and restored full access to `父`, `附`, `覆`, `复`, `扶`, `幅` etc. across multiple pages.
  - Fixed TSF `would_handle` key filter bug to ensure `=` and `-` page-turning keys are properly processed without leaking into host applications.
- **Complete English Alphabet & Irregular Forms**:
  - Restored single-letter completions for all letters `a`..`z` with proper phonetic meanings (e.g. `b` -> `字母B`).
  - Added support for all high-frequency English vocabulary and irregular past tenses / participles (`went`, `took`, `came`, `knew`, `saw`, `got`, `was`, `were`, `been`, `old`, etc.).
  - Deep scanning for short prefixes to prioritize common English words over rare phonetic place names.
- **Bilingual & Multilingual Hierarchical Candidates**: Displays Chinese along with multilingual translations (English, German, Japanese, etc.) hierarchically; supports English-to-Chinese reverse lookup.
- **Single-tap Shift Mode Toggle**: Seamlessly switch between Chinese and ABC English input mode with a single Shift key press, featuring floating status indicator bubbles.
- **Dual Architecture TSF**: Complete Text Services Framework support for both 64-bit and 32-bit Windows host applications.
- **Adaptive Learning**: Context-aware association, dynamic frequency boosting, and automatic user vocabulary learning.

## [v1.0.2]

### Enhancements
- Optimized sentence pinyin segmentation using Viterbi k-best algorithm.
- Enhanced installer scripts and registry cleanups.

## [v1.0.1]

### Improvements
- Integrated CC-CEDICT Chinese-English reverse lookup with offline dictionary.

## [v1.0.0]

### Initial Release
- Initial stable release including standalone hook-based IME and TSF text service.
