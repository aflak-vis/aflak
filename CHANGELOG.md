# Changelog

## [Unreleased]

### Changed
- Update imgui to 0.0.22-pre

### Added
- Show current working directory by default on file selector
- Extract units and WCS from FITS files and keep them during the whole
  pipeline. Show units and real world coordinates on input windows.

### Fixed
- Fix error in file selector
- Fix error on computing texture dimension in Image2d viewer.

## [v0.0.3] - 2018-10-18

- Double-feedback-loop for variables
- FITS export
- Node removal
- Auto-layout of output windows
- Show explanations for each node
- Smoother scrolling on node editor
- Fix constant node names after import
- Miscellaneous internal improvements

## [v0.0.2] - 2018-08-11

- Improve doc

## [v0.0.1] - 2018-07-04

- Very first release
- Tested on GNU/Linux (Ubuntu/Debian), with FITS files from the
  [SDSS MaNGA sky survey](http://www.sdss.org/dr14/manga/)
- Windows/macOS support in progress
