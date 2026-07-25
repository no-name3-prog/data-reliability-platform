import { cn, severityColor, statusColor } from "@/lib/utils";

export function StatusBadge({ status }: { status: string }) {
  return (
    <span
      className={cn(
        "badge ring-1 ring-inset capitalize",
        statusColor(status),
      )}
    >
      {status.replaceAll("_", " ")}
    </span>
  );
}

export function SeverityBadge({ severity }: { severity: string }) {
  return (
    <span className={cn("badge capitalize", severityColor(severity))}>
      {severity}
    </span>
  );
}
