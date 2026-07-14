import { Dashboard } from "./features/dashboard/Dashboard";
import { useWorkspaceController } from "./features/dashboard/useWorkspaceController";

export function App() {
  const controller = useWorkspaceController();
  return <Dashboard controller={controller} />;
}
