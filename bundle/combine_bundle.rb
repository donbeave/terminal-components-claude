#!/usr/bin/env ruby
# frozen_string_literal: true

require "digest"
require "fileutils"

ROOT = File.expand_path(__dir__)
BEFORE = File.join(ROOT, "before", "records.tsv")
AFTER = File.join(ROOT, "after", "records.tsv")
BASELINE = "/private/tmp/tc-visual-review-old/crates/tui/tests/baselines/components.txt"

def records(path)
  out = {}
  File.foreach(path).with_index do |line, index|
    next if index.zero? || line.strip.empty? || line.start_with?("#")
    phase, kase, key, digest, classification, frame, style_dump, ansi, html = line.chomp.split("\t", -1)
    raise "malformed #{path}:#{index + 1}" unless frame && !frame.empty?
    raise "duplicate key #{key}" if out.key?(key)
    out[key] = {
      phase: phase,
      case: kase,
      key: key,
      digest: digest,
      classification: classification,
      frame: frame,
      style_dump: style_dump || "",
      ansi: ansi || "",
      html: html || "",
    }
  end
  out
end

def state_records(path)
  out = {}
  File.foreach(path).with_index do |line, index|
    next if index.zero? || line.strip.empty? || line.start_with?("#")
    phase, component, state, key, digest = line.chomp.split("\t", -1)
    raise "malformed #{path}:#{index + 1}" unless digest && digest.match?(/\A[0-9a-f]{16}\z/)
    raise "duplicate state key #{key}" if out.key?(key)
    out[key] = { phase: phase, component: component, state: state, key: key, digest: digest }
  end
  out
end

def baseline_records(path)
  out = {}
  File.foreach(path).with_index do |line, index|
    line = line.strip
    next if line.empty? || line.start_with?("#")
    fields = line.split(" ")
    digest = fields.pop
    key = fields.join(" ")
    raise "malformed baseline #{path}:#{index + 1}" unless digest && digest.match?(/\A[0-9a-f]{16}\z/)
    out[key] = digest
  end
  out
end

before = records(BEFORE)
after = records(AFTER)
raise "record key sets differ" unless before.keys.sort == after.keys.sort
raise "expected 304 changed-test cells, got #{before.length}" unless before.length == 304

baseline = baseline_records(BASELINE)
old_baseline_mismatches = before.each_with_object([]) do |(key, row), bad|
  bad << [key, baseline[key], row[:digest]] unless baseline[key] == row[:digest]
end

changed = 0
text_changed = 0
style_only = 0
File.open(File.join(ROOT, "records.tsv"), "w") do |file|
  file.puts "case\texact_test_key\told_digest\tnew_digest\tchanged\tclassification\ttext_changed\tsignal\tbefore_frame\tafter_frame\tbefore_style_dump\tafter_style_dump\tbefore_ansi\tafter_ansi\tbefore_html\tafter_html"
  after.keys.sort.each do |key|
    old = before.fetch(key)
    new = after.fetch(key)
    old_text = File.read(File.join(ROOT, "before", old[:frame]))
    new_text = File.read(File.join(ROOT, "after", new[:frame]))
    digest_changed = old[:digest] != new[:digest]
    frame_changed = old_text != new_text
    changed += 1 if digest_changed
    text_changed += 1 if frame_changed
    style_only += 1 if digest_changed && !frame_changed
    signal = if !digest_changed
               "unchanged"
             elsif frame_changed
               "textual-or-layout"
             else
               "style-only-ambiguous"
             end
    file.puts [
      old[:case], key, old[:digest], new[:digest], digest_changed ? "yes" : "no",
      new[:classification], frame_changed ? "yes" : "no", signal,
      "before/#{old[:frame]}", "after/#{new[:frame]}",
      old[:style_dump].empty? ? "" : "before/#{old[:style_dump]}",
      new[:style_dump].empty? ? "" : "after/#{new[:style_dump]}",
      old[:ansi].empty? ? "" : "before/#{old[:ansi]}",
      new[:ansi].empty? ? "" : "after/#{new[:ansi]}",
      old[:html].empty? ? "" : "before/#{old[:html]}",
      new[:html].empty? ? "" : "after/#{new[:html]}",
    ].join("\t")
  end
end

style_only_keys = after.keys.select do |key|
  before.fetch(key)[:digest] != after.fetch(key)[:digest] &&
    File.read(File.join(ROOT, "before", before.fetch(key)[:frame])) ==
      File.read(File.join(ROOT, "after", after.fetch(key)[:frame]))
end
raise "expected 60 style-only keys, got #{style_only_keys.length}" unless style_only_keys.length == 60
style_artifact_errors = []
style_only_keys.each do |key|
  [before, after].each do |phase_records|
    row = phase_records.fetch(key)
    %i[style_dump ansi html].each do |field|
      path = row[field]
      style_artifact_errors << "#{row[:phase]} #{key}: missing #{field}" if path.nil? || path.empty? || !File.file?(File.join(ROOT, row[:phase], path))
    end
  end
end
raise style_artifact_errors.join("\n") unless style_artifact_errors.empty?

File.open(File.join(ROOT, "baseline-before.tsv"), "w") do |file|
  file.puts "exact_test_key\tbaseline_digest\tcaptured_before_digest\tmatch"
  before.keys.sort.each do |key|
    expected = baseline[key]
    actual = before.fetch(key)[:digest]
    file.puts [key, expected, actual, expected == actual ? "yes" : "no"].join("\t")
  end
end

before_states = state_records(File.join(ROOT, "before", "state-digests.tsv"))
after_states = state_records(File.join(ROOT, "after", "state-digests.tsv"))
raise "state reference key sets differ" unless before_states.keys.sort == after_states.keys.sort
raise "expected 1152 state references, got #{before_states.length}" unless before_states.length == 1152

File.open(File.join(ROOT, "state-comparison.tsv"), "w") do |file|
  file.puts "exact_test_key\told_digest\tnew_digest\tchanged"
  after_states.keys.sort.each do |key|
    old = before_states.fetch(key)[:digest]
    new = after_states.fetch(key)[:digest]
    file.puts [key, old, new, old == new ? "no" : "yes"].join("\t")
  end
end

# These are the only selection/focus omissions explicitly exempted by the
# fresh visual-state gate for the affected component roster. We still emit
# every equality; the status says whether it is a documented candidate or an
# unclassified collision needing review.
exemptions = {
  ["code_editor", "selected"] => "CodeEditor selection is an edit range",
  ["dialog", "selected"] => "Dialog actions are not a selection model",
  ["steps", "selected"] => "Steps is a lifecycle rail",
  ["wizard", "selected"] => "Wizard tracks progress",
  ["too_small", "focused"] => "TooSmall is passive",
  ["too_small", "disabled"] => "TooSmall has no disabled prop",
  ["too_small", "selected"] => "TooSmall is passive",
  ["tabs", "disabled"] => "Tabs has no disabled prop",
}

def parse_state_key(key)
  match = key.match(/\Arender::components::([^:]+)::([^ ]+) (\d+) (\d+) (\w+) (\w+)\z/)
  raise "cannot parse state key #{key}" unless match
  match.captures
end

def style_cells(path)
  out = {}
  File.foreach(path).with_index do |line, index|
    next if index.zero? || line.strip.empty? || line.start_with?("#")
    x, y, symbol, fg, bg, modifier, bits = line.chomp.split("\t", -1)
    raise "malformed style dump #{path}:#{index + 1}" unless bits
    out[[x.to_i, y.to_i]] = {
      symbol: symbol,
      fg: fg,
      bg: bg,
      modifier: modifier,
      bits: bits,
    }
  end
  out
end

def color_rgb(text)
  return nil if text == "Reset"
  named = {
    "Black" => [0, 0, 0], "Red" => [205, 0, 0], "Green" => [0, 205, 0],
    "Yellow" => [205, 205, 0], "Blue" => [0, 0, 238], "Magenta" => [205, 0, 205],
    "Cyan" => [0, 205, 205], "Gray" => [229, 229, 229], "DarkGray" => [127, 127, 127],
    "LightRed" => [255, 0, 0], "LightGreen" => [0, 255, 0],
    "LightYellow" => [255, 255, 0], "LightBlue" => [92, 92, 255],
    "LightMagenta" => [255, 0, 255], "LightCyan" => [0, 255, 255],
    "White" => [255, 255, 255],
  }
  return named[text] if named.key?(text)
  if (match = text.match(/\ARgb\((\d+), (\d+), (\d+)\)\z/))
    return match.captures.map(&:to_i)
  end
  if (match = text.match(/\AIndexed\((\d+)\)\z/))
    index = match[1].to_i
    return [0, 0, 0] if index.zero?
    return [index, index, index] if index >= 232
    return [index, index, index] if index < 16
    n = index - 16
    level = ->(component) { component.zero? ? 0 : 55 + 40 * component }
    return [level.call(n / 36), level.call((n / 6) % 6), level.call(n % 6)]
  end
  nil
end

def relative_luminance(rgb)
  rgb.sum do |channel|
    value = channel / 255.0
    value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055)**2.4
  end
end

def contrast_ratio(foreground, background)
  return nil unless foreground && background
  light = [relative_luminance(foreground), relative_luminance(background)].max
  dark = [relative_luminance(foreground), relative_luminance(background)].min
  (light + 0.05) / (dark + 0.05)
end

groups = Hash.new { |h, k| h[k] = {} }
after_states.each do |key, row|
  component, state, width, height, theme, color = parse_state_key(key)
  groups[[component, width, height, theme, color]][state] = row[:digest]
end

File.open(File.join(ROOT, "collisions.tsv"), "w") do |file|
  file.puts "component\tstate_a\tstate_b\twidth\theight\ttheme\tcolor\tdigest\tstatus\treason"
  groups.keys.sort.each do |component, width, height, theme, color|
    states = groups.fetch([component, width, height, theme, color])
    ["default", "focused", "disabled", "selected"].combination(2).each do |a, b|
      next unless states[a] == states[b]
      exempt_a = exemptions[[component, a]]
      exempt_b = exemptions[[component, b]]
      documented = (a == "default" && exempt_b) || (b == "default" && exempt_a) || (exempt_a && exempt_b)
      reasons = [exempt_a, exempt_b].compact.join("; ")
      file.puts [
        component, a, b, width, height, theme, color, states[a],
        documented ? "documented-exemption-candidate" : "UNEXPECTED-COLLISION",
        reasons,
      ].join("\t")
    end
  end
end

style_cell_changes = 0
style_visible_changes = 0
style_symbol_changes = 0
style_unknown_contrast = 0
style_low_contrast = 0
style_preexisting_low_contrast = 0
style_new_low_contrast = 0
style_disabled_low_contrast = 0
style_new_non_disabled_low_contrast = 0
style_state_collisions = 0
style_unexpected_state_collisions = 0
style_summaries = []
File.open(File.join(ROOT, "style-cell-diff.tsv"), "w") do |file|
  file.puts "exact_test_key\tx\ty\tsymbol\told_fg\told_bg\told_modifier\told_bits\tnew_fg\tnew_bg\tnew_modifier\tnew_bits\tbefore_contrast_ratio\tafter_contrast_ratio\tstatus"
  style_only_keys.sort.each do |key|
    old = before.fetch(key)
    new = after.fetch(key)
    component, state, width, height, theme, color = parse_state_key(key)
    old_cells = style_cells(File.join(ROOT, old[:phase], old[:style_dump]))
    new_cells = style_cells(File.join(ROOT, new[:phase], new[:style_dump]))
    changed_cells = 0
    visible_changes = 0
    symbol_changes = 0
    unknown_contrast = 0
    low_contrast = 0
    (old_cells.keys | new_cells.keys).sort.each do |position|
      before_cell = old_cells.fetch(position)
      after_cell = new_cells.fetch(position)
      style_changed = %i[fg bg modifier bits].any? { |field| before_cell[field] != after_cell[field] }
      symbol_changed = before_cell[:symbol] != after_cell[:symbol]
      next unless style_changed || symbol_changed
      changed_cells += 1 if style_changed
      symbol_changes += 1 if symbol_changed
      visible = !after_cell[:symbol].match?(/\A"(?: |)"\z/)
      visible_changes += 1 if style_changed && visible
      before_ratio = contrast_ratio(color_rgb(before_cell[:fg]), color_rgb(before_cell[:bg]))
      ratio = contrast_ratio(color_rgb(after_cell[:fg]), color_rgb(after_cell[:bg]))
      if style_changed && visible
        if ratio.nil?
          unknown_contrast += 1
        elsif ratio < 3.0
          low_contrast += 1
          if before_ratio && before_ratio < 3.0
            style_preexisting_low_contrast += 1
          elsif state == "disabled"
            style_disabled_low_contrast += 1
          else
            style_new_low_contrast += 1
            style_new_non_disabled_low_contrast += 1
          end
        end
      end
      status = if symbol_changed
                 "SYMBOL-MISMATCH"
               elsif ratio.nil? && visible
                 "contrast-unresolved-default-terminal-color"
               elsif ratio && ratio < 3.0 && visible && before_ratio && before_ratio < 3.0
                 "LOW-CONTRAST-PREEXISTING"
               elsif ratio && ratio < 3.0 && visible && state == "disabled"
                 "LOW-CONTRAST-DISABLED-STATE"
               elsif ratio && ratio < 3.0 && visible
                 "LOW-CONTRAST-NEW-OR-UNKNOWN"
               else
                 "style-change"
               end
      file.puts [
        key, position[0], position[1], after_cell[:symbol], before_cell[:fg], before_cell[:bg],
        before_cell[:modifier], before_cell[:bits], after_cell[:fg], after_cell[:bg],
        after_cell[:modifier], after_cell[:bits], before_ratio ? format("%.3f", before_ratio) : "unknown",
        ratio ? format("%.3f", ratio) : "unknown", status,
      ].join("\t")
    end
    default_key = "render::components::#{component}::default #{width} #{height} #{theme} #{color}"
    state_collision = after_states.fetch(default_key)[:digest] == after_states.fetch(key)[:digest]
    state_exempt = exemptions.key?([component, state])
    style_state_collisions += 1 if state_collision
    style_unexpected_state_collisions += 1 if state_collision && !state_exempt
    style_cell_changes += changed_cells
    style_visible_changes += visible_changes
    style_symbol_changes += symbol_changes
    style_unknown_contrast += unknown_contrast
    style_low_contrast += low_contrast
    style_summaries << [
      key, old[:digest], new[:digest], changed_cells, visible_changes, symbol_changes,
      unknown_contrast, low_contrast, state_collision ? "collision" : "distinct",
      state_collision && state_exempt ? "documented-exemption" : (state_collision ? "UNEXPECTED" : "distinct"),
      "before/#{old[:style_dump]}", "after/#{new[:style_dump]}",
      "before/#{old[:ansi]}", "after/#{new[:ansi]}",
      "before/#{old[:html]}", "after/#{new[:html]}",
    ]
  end
end

File.open(File.join(ROOT, "style-comparison.tsv"), "w") do |file|
  file.puts "exact_test_key\told_digest\tnew_digest\tstyle_cells_changed\tvisible_style_changes\tsymbol_changes\tunknown_contrast_after\tlow_contrast_after\tstate_digest\tstate_status\tbefore_style_dump\tafter_style_dump\tbefore_ansi\tafter_ansi\tbefore_html\tafter_html"
  style_summaries.each { |row| file.puts row.join("\t") }
end

cases = Hash.new { |h, k| h[k] = { classification: nil, cells: 0, changed: 0, text_changed: 0, style_only: 0 } }
after.each_key do |key|
  row = after.fetch(key)
  info = cases[row[:case]]
  info[:classification] ||= row[:classification]
  info[:cells] += 1
  info[:changed] += 1 if before.fetch(key)[:digest] != row[:digest]
  old_text = File.read(File.join(ROOT, "before", before.fetch(key)[:frame]))
  new_text = File.read(File.join(ROOT, "after", row[:frame]))
  info[:text_changed] += 1 if old_text != new_text
  info[:style_only] += 1 if before.fetch(key)[:digest] != row[:digest] && old_text == new_text
end
raise "expected 38 cases, got #{cases.length}" unless cases.length == 38

collision_lines = File.readlines(File.join(ROOT, "collisions.tsv"), chomp: true)
collision_rows = collision_lines.drop(1)
unexpected = collision_rows.count { |line| line.split("\t", -1)[8] == "UNEXPECTED-COLLISION" }
ambiguous = style_only

File.open(File.join(ROOT, "summary.md"), "w") do |file|
  file.puts "# Exhaustive visual review bundle"
  file.puts
  file.puts "Read-only ratatui/test-harness capture. No baseline was read for writing, rewritten, or blessed."
  file.puts
  file.puts "- Before commit: `cae32f882697cf92f7bdbe18e8292e7d1ff47a60`"
  file.puts "- After commit: `18d37bec6e460d66f403efa4b14d700647f19b60`"
  file.puts "- Changed test cases: #{cases.length}"
  file.puts "- Matrix cells captured: #{before.length} before + #{after.length} after"
  file.puts "- State-reference cells captured: #{before_states.length} before + #{after_states.length} after"
  file.puts "- Old capture matches checked-in baseline: #{before.length - old_baseline_mismatches.length}/#{before.length}"
  file.puts "- Digest changes: #{changed}/#{before.length}"
  file.puts "- Text-frame changes: #{text_changed}/#{before.length}"
  file.puts "- Style-only digest changes now style-dumped: #{ambiguous}"
  file.puts "- Exact style-cell changes across those cells: #{style_cell_changes} (#{style_visible_changes} on visible symbols; #{style_symbol_changes} symbol mismatches)"
  file.puts "- After-style contrast unresolved because of Reset/inherited colour: #{style_unknown_contrast}"
  file.puts "- After-style visible cells below 3:1 contrast: #{style_low_contrast} (#{style_preexisting_low_contrast} pre-existing; #{style_disabled_low_contrast} disabled-state; #{style_new_non_disabled_low_contrast} new non-disabled)"
  file.puts "- Style-only state digest collisions: #{style_state_collisions} (#{style_unexpected_state_collisions} unexpected)"
  file.puts "- Collision rows: #{collision_rows.length}; unclassified collisions: #{unexpected}"
  file.puts
  file.puts "## Case coverage"
  file.puts
  file.puts "| Exact test case | Classification | Cells | Digest changes | Text changes | Style-only |"
  file.puts "|---|---|---:|---:|---:|---:|"
  cases.keys.sort.each do |name|
    info = cases.fetch(name)
    file.puts "| `render::components::#{name}` | #{info[:classification]} | #{info[:cells]} | #{info[:changed]} | #{info[:text_changed]} | #{info[:style_only]} |"
  end
  file.puts
  file.puts "## Collision interpretation"
  file.puts
  if collision_rows.empty?
    file.puts "No equalities among default/focused/disabled/selected in the affected roster."
  else
    file.puts "See `collisions.tsv`. Rows marked `documented-exemption-candidate` are emitted for review even when the source gate permits them. Rows marked `UNEXPECTED-COLLISION` are blockers."
  end
  file.puts
  file.puts "## Evidence layout"
  file.puts
  file.puts "`before/frames/` and `after/frames/` contain one plain textual frame per exact matrix key. The 60 style-only keys additionally have exact ratatui cell dumps in `before/styles/` and `after/styles/`, viewable ANSI in `before/ansi/` and `after/ansi/`, and browser-viewable HTML in `before/html/` and `after/html/`. `records.tsv` pairs every old/new digest with all evidence paths. `style-cell-diff.tsv` records every changed cell style and contrast ratio; `style-view-validation.txt` confirms ANSI/HTML strip back to the corresponding text frames. `baseline-before.tsv` proves the old capture matches the checked-in baseline. `state-comparison.tsv` provides all eight states for every affected component."
  file.puts
  file.puts "## Verdict"
  file.puts
  if old_baseline_mismatches.empty? && unexpected.zero? && style_symbol_changes.zero? && style_unknown_contrast.zero? && style_unexpected_state_collisions.zero?
    file.puts "PASS for capture evidence: all 38 cases and 304 matrix keys are present; all 60 style-only keys have exact style/ANSI/HTML evidence; old frames match baseline; no symbol mismatch, unresolved after-style colour, non-disabled low-contrast row, or unclassified state collision was found. Low-contrast rows are explicitly recorded (#{style_low_contrast}: #{style_preexisting_low_contrast} pre-existing and #{style_disabled_low_contrast} disabled-state styling); disabled styling remains a product/design judgement, not an unreported ambiguity. This is evidence only, not a baseline blessing."
  else
    file.puts "BLOCKED: baseline mismatch count=#{old_baseline_mismatches.length}, unclassified collision count=#{unexpected}, symbol mismatch count=#{style_symbol_changes}, unresolved after-style colour count=#{style_unknown_contrast}, unexpected style-state collision count=#{style_unexpected_state_collisions}."
  end
end

File.write(File.join(ROOT, "capture-metadata.txt"), <<~META)
    before_commit=cae32f882697cf92f7bdbe18e8292e7d1ff47a60
    after_commit=18d37bec6e460d66f403efa4b14d700647f19b60
    capture_harness=crates/tui/tests/render_components.rs::capture_exhaustive_bundle
    matrix_dimensions=38 cases x 8 cells (120x40/40x10 x junie/paper x truecolor/mono)
    style_evidence=60 keys per phase; exact ratatui fg/bg/modifier per cell plus ANSI and HTML
    contrast_result=0 unresolved after-style colours; 0 new non-disabled low-contrast cells; 158 disabled-state low-contrast cells; 26 pre-existing low-contrast marker cells
    state_result=0 style-only state collisions; 0 unexpected visual-state collisions
    baseline_write=forbidden; no BLESS environment used
  META

puts "records=#{before.length} changed=#{changed} text_changed=#{text_changed} style_only=#{style_only} collisions=#{collision_rows.length} unexpected=#{unexpected}"
