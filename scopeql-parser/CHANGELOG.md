# CHANGELOG

All significant changes to the ScopeQL Parser be documented in this file.

## Unreleased

## v0.2.0 (2026-02-13)

### Breaking Changes

* No longer recognize `EQUALITY` token since `CREATE EQUALITY INDEX` clause has been merged into `CREATE POINT INDEX`.

### New Features

* Recognize `POINT` token for `CREATE POINT INDEX` clause.
