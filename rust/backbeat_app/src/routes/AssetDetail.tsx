import { useParams } from "@solidjs/router";
import { convertFileSrc } from "@tauri-apps/api/core";
import { createEffect, createMemo, createResource, createSignal, on, Show } from "solid-js";

import { BackLink } from "../components/BackLink";
import DetailsGrid from "../components/DetailsGrid";
import { HashField } from "../components/HashField";
import Lister from "../components/Lister";
import { PageHeader } from "../components/PageHeader";
import Banner from "../components/primitives/Banner";
import Button from "../components/primitives/Button";
import Section from "../components/primitives/Section";
import { formatBytes } from "../lib/format";
import { getAssetDetail, getAssetPreview, openAssetFolder } from "../lib/tauri";
import styles from "./AssetDetail.module.css";

type TextEncoding = "euc-kr" | "shift_jis" | "utf-8";

function previewSrc(
	path: null | string,
	mimeType: string,
	dataBase64: null | string,
): null | string {
	if (path) {
		return convertFileSrc(path);
	}
	if (dataBase64) {
		return `data:${mimeType};base64,${dataBase64}`;
	}
	return null;
}

function decodeText(dataBase64: string, encoding: TextEncoding): string {
	const binary = atob(dataBase64);
	const bytes = new Uint8Array(binary.length);
	for (let index = 0; index < binary.length; index += 1) {
		bytes[index] = binary.charCodeAt(index);
	}
	return new TextDecoder(encoding).decode(bytes);
}

function detectTextEncoding(dataBase64: string): TextEncoding {
	for (const encoding of ["utf-8", "shift_jis", "euc-kr"] as const) {
		if (!decodeText(dataBase64, encoding).includes("�")) {
			return encoding;
		}
	}
	return "utf-8";
}

function firstFilename(path: string | undefined): string | undefined {
	return path?.slice(path.lastIndexOf("/") + 1);
}

export default function AssetDetail() {
	const params = useParams();
	const [detail] = createResource(() => params.assetId, getAssetDetail);
	const [preview] = createResource(() => detail()?.id, getAssetPreview);
	const [openError, setOpenError] = createSignal<null | string>(null);
	const [textEncoding, setTextEncoding] = createSignal<TextEncoding>("utf-8");

	createEffect(
		on(
			() => {
				const p = preview();
				return p?.kind === "text" ? p.data_base64 : null;
			},
			(data) => {
				if (data) {
					setTextEncoding(detectTextEncoding(data));
				}
			},
		),
	);

	const mediaSrc = createMemo(() => {
		const p = preview();
		if (p?.kind !== "image" && p?.kind !== "audio") {
			return null;
		}
		return previewSrc(p.path, p.mime_type, p.data_base64);
	});
	const textPreview = createMemo(() => {
		const p = preview();
		return p?.kind === "text" && p.data_base64
			? decodeText(p.data_base64, textEncoding())
			: null;
	});

	async function handleOpen(assetId: string) {
		setOpenError(null);
		try {
			await openAssetFolder(assetId);
		} catch (err) {
			setOpenError(String(err));
		}
	}

	return (
		<>
			<p class="breadcrumb">
				<BackLink />
			</p>

			<Show when={detail.loading}>
				<Banner>Loading asset…</Banner>
			</Show>

			<Show when={detail.error}>
				<Banner variant="danger">Failed to load asset: {String(detail.error)}</Banner>
			</Show>

			<Show when={detail()}>
				{(asset) => (
					<>
						<PageHeader
							actions={
								<Show when={asset().size !== null}>
									<Button
										onClick={() => handleOpen(asset().id)}
										type="button"
										variant="base"
									>
										Reveal in file browser
									</Button>
								</Show>
							}
							title={`Asset: ${firstFilename(asset().dependents[0]?.path) ?? asset().id}`}
						/>

						<Show when={openError()}>
							<Banner variant="danger">Couldn't open folder: {openError()}</Banner>
						</Show>

						<Section heading="Asset Details">
							<DetailsGrid>
								{(Field) => (
									<>
										<Field label="Size">
											<Show
												fallback={<span class="error">missing</span>}
												when={asset().size !== null}
											>
												{formatBytes(asset().size!)}
											</Show>
										</Field>
										<Field label="Depended on by">
											{asset().dependents.length}{" "}
											{asset().dependents.length === 1 ? "chart" : "charts"}
										</Field>
										<Show when={preview()?.mime_type}>
											{(mime) => (
												<Field label="Type">
													<span class="mono">{mime()}</span>
												</Field>
											)}
										</Show>
									</>
								)}
							</DetailsGrid>

							<HashField label="SHA-256" value={asset().id} />

							<Show when={preview.loading}>
								<Banner>Loading preview…</Banner>
							</Show>

							<Show when={mediaSrc()}>
								{(src) => (
									<div class={styles.preview}>
										<Show when={preview()?.kind === "image"}>
											<img
												alt="Asset preview"
												class={styles.image}
												src={src()}
											/>
										</Show>
										<Show when={preview()?.kind === "audio"}>
											<audio class={styles.audio} controls src={src()}>
												Your browser does not support audio playback.
											</audio>
										</Show>
									</div>
								)}
							</Show>
							<Show when={textPreview() !== null}>
								<div class={styles.textPreview}>
									<pre class={styles.text}>{textPreview()}</pre>
								</div>
							</Show>
						</Section>

						<Section heading="Charts depending on this asset">
							<Show
								fallback={
									<div class="empty-state empty-state-compact">
										No charts currently depend on this asset.
									</div>
								}
								when={asset().dependents.length > 0}
							>
								<Lister
									content={(dep) => dep.description}
									detail={(dep) => <div class={styles.depPath}>{dep.path}</div>}
									href={(dep) => `/bundles/${dep.bundle_id}`}
									items={asset().dependents}
								/>
							</Show>
						</Section>
					</>
				)}
			</Show>
		</>
	);
}
