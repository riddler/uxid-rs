### Added

- `is_valid_with(id, delimiter, prefix)` answers whether an id decodes under `delimiter` and, given `Some(prefix)`, carries exactly that prefix (`None` or an empty prefix accepts any prefix or none); `is_deterministic_with(id, delimiter)` answers whether an id decodes under `delimiter` to a deterministic body. Both read no clock, need no feature, and answer `false` under an empty delimiter. `is_valid(id)` and `is_deterministic(id)` are these with the delimiter `_` and no required prefix.
