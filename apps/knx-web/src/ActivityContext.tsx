/** Provides exactly one activity observer for the mounted editor application. */
import { createContext, useContext, useEffect, useState, useSyncExternalStore, type ReactNode } from "react";
import { ActivityController } from "./activityController";

const Context = createContext<ActivityController | null>(null);
export function ActivityProvider({ children }: { children: ReactNode }) {
  const [controller] = useState(() => new ActivityController());
  useEffect(controller.connect, [controller]);
  return <Context.Provider value={controller}>{children}</Context.Provider>;
}
export function useActivityController(): ActivityController {
  const controller = useContext(Context);
  if (!controller) throw new Error("Activities require the App-root provider");
  return controller;
}
export function useActivities() {
  const controller = useActivityController();
  return useSyncExternalStore(controller.subscribe, controller.getSnapshot);
}
