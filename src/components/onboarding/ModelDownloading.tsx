import React, { useEffect, useMemo } from "react";
import SpoknWordmark from "../icons/SpoknWordmark";
import { useModelStore } from "../../stores/modelStore";
import { toast } from "sonner";

interface ModelDownloadingProps {
  modelId: string;
  onComplete: () => void;
}

/* Friendly, model-name-free download screen. Kicks off the download for the
 * resolved model on mount, shows progress, transitions out once the model is
 * downloaded + verified + selected. No exposure of Whisper/Parakeet
 * internals — casual users don't need to care. */
const ModelDownloading: React.FC<ModelDownloadingProps> = ({ modelId, onComplete }) => {
  const {
    models,
    downloadModel,
    selectModel,
    downloadingModels,
    verifyingModels,
    extractingModels,
    downloadProgress,
    downloadStats,
  } = useModelStore();

  const model = models.find((m) => m.id === modelId);
  const isDownloading = modelId in downloadingModels;
  const isVerifying = modelId in verifyingModels;
  const isExtracting = modelId in extractingModels;
  const progress = downloadProgress[modelId];
  const stats = downloadStats[modelId];

  // Kick off the download on mount (or skip straight to select if already there)
  useEffect(() => {
    let cancelled = false;
    (async () => {
      if (!model) return;
      if (model.is_downloaded) {
        const ok = await selectModel(modelId);
        if (!cancelled && ok) onComplete();
        return;
      }
      if (!isDownloading) {
        const ok = await downloadModel(modelId);
        if (!ok && !cancelled) {
          toast.error("Download failed. Check your internet connection.");
        }
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [modelId, model?.is_downloaded]);

  // Once fully downloaded + post-processed, select + continue
  useEffect(() => {
    if (!model) return;
    const done =
      model.is_downloaded && !isDownloading && !isVerifying && !isExtracting;
    if (done) {
      selectModel(modelId).then((ok) => {
        if (ok) onComplete();
      });
    }
  }, [
    model?.is_downloaded,
    isDownloading,
    isVerifying,
    isExtracting,
    modelId,
    model,
    selectModel,
    onComplete,
  ]);

  const pct = useMemo(() => {
    if (model?.is_downloaded) return 100;
    if (progress && progress.percentage > 0) return progress.percentage;
    return 0;
  }, [progress, model?.is_downloaded]);

  const statusLabel = useMemo(() => {
    if (isVerifying) return "Verifying download…";
    if (isExtracting) return "Unpacking…";
    if (model?.is_downloaded) return "Ready";
    if (isDownloading && progress) return "Downloading speech engine…";
    return "Preparing…";
  }, [isVerifying, isExtracting, model?.is_downloaded, isDownloading, progress]);

  const sizeLabel = useMemo(() => {
    if (!progress) return "";
    const mb = (n: number) => (n / 1024 / 1024).toFixed(0);
    const speed = stats?.speed ? ` · ${stats.speed.toFixed(1)} MB/s` : "";
    return `${mb(progress.downloaded)} / ${mb(progress.total)} MB${speed}`;
  }, [progress, stats]);

  return (
    <div
      className="h-screen w-screen flex flex-col items-center justify-center p-6 gap-6"
      style={{ background: "var(--color-spokn-bg)" }}
    >
      {/* eslint-disable i18next/no-literal-string */}
      <div className="flex flex-col items-center gap-3">
        <SpoknWordmark variant="hero" />
        <p className="text-spokn-text-2 text-sm max-w-md text-center">
          Setting things up for you. This one-time download will take a moment
          — future launches are instant.
        </p>
      </div>

      <div className="w-full max-w-md flex flex-col gap-3">
        <div className="h-2 w-full rounded-full bg-spokn-surface overflow-hidden border border-spokn-hairline">
          <div
            className="h-full transition-all duration-300 ease-out"
            style={{
              width: `${pct}%`,
              background: "var(--spokn-accent-grad)",
            }}
          />
        </div>
        <div className="flex items-center justify-between text-xs font-mono tracking-wider">
          <span className="text-spokn-text-2">{statusLabel}</span>
          <span className="text-spokn-text-3">
            {pct.toFixed(0)}%
            {sizeLabel && ` · ${sizeLabel}`}
          </span>
        </div>
      </div>
      {/* eslint-enable i18next/no-literal-string */}
    </div>
  );
};

export default ModelDownloading;
