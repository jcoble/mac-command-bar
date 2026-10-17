// What the Workflows view shows under a step comes from the step's structured
// output, the same facts get_workflow_run gives agents. Logic only.
import assert from 'node:assert/strict';
import { workflowStepFacts } from '../src/lib/shell/workflows/workflowTypes.ts';

{ // step_facts_come_from_structured_output: a check's commands, a review's verdict, a PR step's link
  const check = workflowStepFacts({ structuredOutput: { commands: [
    { command: 'pytest', exitCode: 1, output: 'a\nb\nc\nd\ne\nf\nFAILED test_add\n' },
    { command: 'ruff check', exitCode: 0, output: '' }
  ] } });
  assert.deepEqual(check.commands, [
    { command: 'pytest', exitCode: 1, lastLines: 'c\nd\ne\nf\nFAILED test_add' },
    { command: 'ruff check', exitCode: 0, lastLines: '' }
  ]);
  assert.equal(check.verdict, null);

  const review = workflowStepFacts({ structuredOutput: { verdict: 'fail', findings: ['add() ignores b'], summary: 'x' } });
  assert.deepEqual([review.verdict, review.findings, review.commands], ['fail', ['add() ignores b'], []]);

  assert.equal(workflowStepFacts({ structuredOutput: { url: 'https://github.com/o/r/pull/7' } }).prUrl, 'https://github.com/o/r/pull/7');
  assert.deepEqual(workflowStepFacts({ structuredOutput: null }), { commands: [], verdict: null, findings: [], prUrl: null });
}

console.log('workflowStepFacts tests passed');
