export interface CheckpointStep {
  label: string;
  description: string;
}

interface CheckpointStepperProps {
  activeStep: number;
  furthestStep: number;
  steps: CheckpointStep[];
  onSelect: (step: number) => void;
}

export function CheckpointStepper({
  activeStep,
  furthestStep,
  steps,
  onSelect,
}: CheckpointStepperProps) {
  return (
    <nav className="checkpoint-stepper" aria-label="체크포인트 기록 단계">
      <ol>
        {steps.map((step, index) => {
          const reachable = index <= furthestStep;
          return (
            <li className={index < activeStep ? "completed" : ""} key={step.label}>
              <button
                aria-current={index === activeStep ? "step" : undefined}
                disabled={!reachable}
                onClick={() => onSelect(index)}
                type="button"
              >
                <span className="checkpoint-step-number">{index + 1}</span>
                <span>
                  <strong>{step.label}</strong>
                  <small>{step.description}</small>
                </span>
              </button>
            </li>
          );
        })}
      </ol>
    </nav>
  );
}
