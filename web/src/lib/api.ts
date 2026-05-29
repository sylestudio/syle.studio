// Build-time client for the syle public API. Resilient by design: if the API
// is unreachable during a build (e.g. CI without the backend up), pages fall
// back to empty rather than failing the build.

const API_BASE = import.meta.env.API_BASE ?? "http://127.0.0.1:8080";

export interface ImageVariant {
  format: "avif" | "jpeg";
  width: number;
  path: string;
}

export interface Photo {
  id: string;
  gallery_id: string;
  alt: string;
  thumbhash: string;
  width: number;
  height: number;
  position: number;
  variants: ImageVariant[];
}

export interface Gallery {
  id: string;
  slug: string;
  title: string;
  position: number;
  published: boolean;
  description: string;
  notes: string;
  category: string;
  year: number | null;
}

export interface GalleryDetail {
  gallery: Gallery;
  photos: Photo[];
}

export interface BlogPost {
  id: string;
  slug: string;
  title: string;
  /** Structured block document (source of record). */
  blocks: unknown[];
  /** Server-rendered HTML from `blocks`; identical to the CRM preview. */
  body_html: string;
  status: "draft" | "published";
  published_at: number | null;
}

async function get<T>(path: string, fallback: T): Promise<T> {
  try {
    const res = await fetch(`${API_BASE}${path}`);
    if (!res.ok) return fallback;
    return (await res.json()) as T;
  } catch {
    return fallback;
  }
}

export const listGalleries = () =>
  get<Gallery[]>("/api/public/galleries", []);

export const getGallery = (slug: string) =>
  get<GalleryDetail | null>(`/api/public/galleries/${slug}`, null);

export const listPosts = () => get<BlogPost[]>("/api/public/posts", []);

export const getPost = (slug: string) =>
  get<BlogPost | null>(`/api/public/posts/${slug}`, null);
