import { AutoProcessor, RawImage, SamModel, Tensor } from '@huggingface/transformers';

export const MODEL_ID = 'Xenova/slimsam-77-uniform';

export interface PromptPoint {
  x: number;
  y: number;
  label: 0 | 1;
}

export interface MaskCandidate {
  mask: Uint8Array;
  score: number;
}

type Device = 'webgpu' | 'wasm' | 'cpu';

export class Segmenter {
  private embeddings: Record<string, Tensor> | null = null;
  private inputs: any = null;
  private size = { width: 0, height: 0 };

  private constructor(
    private model: any,
    private processor: any,
  ) {}

  static async load(device?: Device, onProgress?: (info: any) => void): Promise<Segmenter> {
    const opts: Record<string, unknown> = { progress_callback: onProgress };
    if (device) opts.device = device;
    const [model, processor] = await Promise.all([
      SamModel.from_pretrained(MODEL_ID, { ...opts, dtype: 'fp32' } as any),
      AutoProcessor.from_pretrained(MODEL_ID, { progress_callback: onProgress } as any),
    ]);
    return new Segmenter(model, processor);
  }

  async setImage(image: RawImage): Promise<void> {
    this.size = { width: image.width, height: image.height };
    this.inputs = await this.processor(image);
    this.embeddings = await this.model.get_image_embeddings(this.inputs);
  }

  async predict(points: PromptPoint[]): Promise<MaskCandidate[]> {
    if (!this.embeddings || !this.inputs) throw new Error('setImage를 먼저 호출해야 합니다');
    const [rh, rw] = this.inputs.reshaped_input_sizes[0];
    const coords = points.flatMap((p) => [p.x * rw, p.y * rh]);
    const input_points = new Tensor('float32', coords, [1, 1, points.length, 2]);
    const input_labels = new Tensor(
      'int64',
      points.map((p) => BigInt(p.label)),
      [1, 1, points.length],
    );
    const outputs = await this.model({ ...this.embeddings, input_points, input_labels });
    const masks: Tensor = (
      await this.processor.post_process_masks(
        outputs.pred_masks,
        this.inputs.original_sizes,
        this.inputs.reshaped_input_sizes,
      )
    )[0];
    const { width, height } = this.size;
    const n = width * height;
    const data = masks.data as Uint8Array;
    const scores = outputs.iou_scores.data as Float32Array;
    const count = masks.dims[1];
    const out: MaskCandidate[] = [];
    for (let i = 0; i < count; i++) {
      out.push({ mask: data.slice(i * n, (i + 1) * n), score: scores[i] });
    }
    return out;
  }
}
