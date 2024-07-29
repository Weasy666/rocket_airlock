# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] - 2024-07-29
### Added
- This file! 🚀
- Added associated type `Error` to `Hatch` and `Communicator`

### Changed
- Consume and return `Rocket<Build>` in `Hatch::from` and `Communicator::from`. This enables an implementor of those traits to modify the `Rocket` instance in the same way it is possible in a `Fairing`.

