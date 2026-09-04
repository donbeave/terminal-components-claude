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
    phase, kase, key, digest, classification, frame = line.chomp.split("\t", -1)
    raise "malformed #{path}:#{index + 1}" unless frame && !frame.empty?
    raise "duplicate key #{key}" if out.key?(key)
    out[key] = {
      phase: phase,
      case: kase,
      key: key,
      digest: digest,
      classification: classification,
      frame: frame
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
  file.puts "case\texact_test_key\told_digest\tnew_digest\tchanged\tclassification\ttext_changed\tsignal\tbefore_frame\tafter_frame"
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
    ].join("\t")
  end
end

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
  file.puts "- Style-only digest changes (ambiguous without cell styles): #{ambiguous}"
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
  file.puts "`before/frames/` and `after/frames/` contain one plain textual frame per exact matrix key. `records.tsv` pairs every old/new digest with both frame paths. `baseline-before.tsv` proves the old capture matches the checked-in baseline. `state-comparison.tsv` provides all eight states for every affected component."
  file.puts
  file.puts "## Verdict"
  file.puts
  if old_baseline_mismatches.empty? && unexpected.zero?
    file.puts "PASS for capture evidence: all 38 cases and 304 matrix keys are present; old frames match baseline; no unclassified state collision was found. This is evidence only, not a baseline blessing."
  else
    file.puts "BLOCKED: baseline mismatch count=#{old_baseline_mismatches.length}, unclassified collision count=#{unexpected}."
  end
end

File.write(File.join(ROOT, "capture-metadata.txt"), <<~META)
    before_commit=cae32f882697cf92f7bdbe18e8292e7d1ff47a60
    after_commit=18d37bec6e460d66f403efa4b14d700647f19b60
    capture_harness=crates/tui/tests/render_components.rs::capture_exhaustive_bundle
    matrix_dimensions=38 cases x 8 cells (120x40/40x10 x junie/paper x truecolor/mono)
    baseline_write=forbidden; no BLESS environment used
  META

puts "records=#{before.length} changed=#{changed} text_changed=#{text_changed} style_only=#{style_only} collisions=#{collision_rows.length} unexpected=#{unexpected}"
