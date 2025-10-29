# Proposal 07: Configuration System for Analysis Settings

## Problem

Users need to customize analysis behavior without modifying source code:

1. **Fixed Thresholds**: Confidence levels, complexity thresholds, and safety levels are hardcoded
2. **No User Control**: Cannot adjust suggestion generation to project needs
3. **One-Size-Fits-All**: Same settings apply to all projects regardless of requirements
4. **Limited Flexibility**: Cannot disable specific suggestion types or categories

Example scenarios requiring configuration:
- Strict projects want only high-confidence suggestions
- Legacy codebases need relaxed thresholds initially
- Teams want to disable certain suggestion types
- CI/CD needs different settings than interactive use

## Solution(s)

### 1. Configuration File Structure

Create `.typemill/analysis.toml`:

```toml
[suggestions]
min_confidence = 0.7  # Minimum confidence threshold (0.0-1.0)
include_safety_levels = ["safe", "requires_review"]
max_per_finding = 3
generate_refactor_calls = true

[thresholds]
max_complexity = 15
max_nesting_depth = 4
max_function_lines = 100
max_parameters = 5

[analysis]
enable_dead_code = true
enable_code_smells = true
enable_complexity = true
enable_maintainability = true
```text
### 2. Configuration Loading

Add configuration layer in `mill-services`:

```rust
pub struct AnalysisConfig {
    pub suggestions: SuggestionConfig,
    pub thresholds: ThresholdConfig,
    pub analysis: AnalysisConfig,
}

impl AnalysisConfig {
    pub fn load() -> Result<Self> {
        // 1. Load from .typemill/analysis.toml
        // 2. Merge with environment variables
        // 3. Apply defaults
    }
}
```text
### 3. Preset Configurations

Built-in presets for common scenarios:

- `strict`: Only safe suggestions, high confidence
- `default`: Balanced settings
- `relaxed`: All suggestions, low confidence
- `ci`: Optimized for CI/CD pipelines

### 4. Environment Variable Overrides

Support `TYPEMILL_ANALYSIS_*` environment variables:

```bash
TYPEMILL_ANALYSIS_MIN_CONFIDENCE=0.8
TYPEMILL_ANALYSIS_MAX_COMPLEXITY=10
```text
## Checklists

### Configuration Schema
- [ ] Define `AnalysisConfig` struct with all settings
- [ ] Define `SuggestionConfig` for suggestion generation
- [ ] Define `ThresholdConfig` for complexity/quality thresholds
- [ ] Add TOML serialization/deserialization

### Configuration Loading
- [ ] Implement config file loading from `.typemill/analysis.toml`
- [ ] Add environment variable overrides
- [ ] Add preset configurations (strict/default/relaxed/ci)
- [ ] Add validation for config values

### Integration
- [ ] Update `analyze.quality` to use config
- [ ] Update `analyze.dead_code` to use config
- [ ] Update suggestion generation to use config
- [ ] Update all analysis tools to respect thresholds

### CLI Commands
- [ ] Add `mill config show` to display current config
- [ ] Add `mill config init --preset <name>` to generate config file
- [ ] Add `mill config validate` to check config validity
- [ ] Add config path to `mill status` output

### Documentation
- [ ] Document configuration file format
- [ ] Document all available settings
- [ ] Document preset configurations
- [ ] Add examples for common scenarios

### Testing
- [ ] Test config file loading
- [ ] Test environment variable overrides
- [ ] Test preset configurations
- [ ] Test invalid config handling
- [ ] Test config validation

## Success Criteria

1. **User Customization**: Users can create `.typemill/analysis.toml` and customize all thresholds
2. **Preset Support**: Built-in presets work correctly (strict/default/relaxed/ci)
3. **Environment Overrides**: Environment variables override config file settings
4. **Validation**: Invalid configs are caught with helpful error messages
5. **CLI Integration**: `mill config` commands work for viewing/creating/validating configs
6. **Backward Compatibility**: Works with default values when no config exists

## Benefits

1. **Flexibility**: Users control analysis behavior per project
2. **Consistency**: Same settings across team via committed config file
3. **CI/CD Ready**: Different settings for interactive vs automated use
4. **Progressive Adoption**: Start relaxed, tighten gradually
5. **Transparency**: Explicit settings visible in config file
6. **No Code Changes**: Adjust behavior without modifying TypeMill source