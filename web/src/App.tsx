import { Navigate, Route, Routes } from "react-router-dom";
import { Layout } from "@/components/Layout";
import { OverviewPage } from "@/pages/Overview";
import { SourcesPage } from "@/pages/Sources";
import { DatasetsPage } from "@/pages/Datasets";
import { DatasetDetailPage } from "@/pages/DatasetDetail";
import { ProfilingPage } from "@/pages/Profiling";
import { ValidationPage } from "@/pages/Validation";
import { LineagePage } from "@/pages/Lineage";
import { IncidentsPage } from "@/pages/Incidents";
import { IncidentDetailPage } from "@/pages/IncidentDetail";

export default function App() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route index element={<OverviewPage />} />
        <Route path="sources" element={<SourcesPage />} />
        <Route path="datasets" element={<DatasetsPage />} />
        <Route path="datasets/:id" element={<DatasetDetailPage />} />
        <Route path="profiling" element={<ProfilingPage />} />
        <Route path="validation" element={<ValidationPage />} />
        <Route path="lineage" element={<LineagePage />} />
        <Route path="incidents" element={<IncidentsPage />} />
        <Route path="incidents/:id" element={<IncidentDetailPage />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  );
}
