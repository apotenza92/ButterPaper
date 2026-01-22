import { invoke } from '@tauri-apps/api/core';

// Response types

export interface OpenPdfResponse {
  success: boolean;
  page_count: number;
  title: string | null;
  error: string | null;
}

export interface RenderResponse {
  success: boolean;
  image_base64: string;
  width: number;
  height: number;
  error: string | null;
}

export interface NavigateResponse {
  success: boolean;
  current_page: number;
  error: string | null;
}

export interface ZoomResponse {
  success: boolean;
  zoom_percent: number;
  error: string | null;
}

export interface PageDimensionsResponse {
  success: boolean;
  width: number;
  height: number;
  error: string | null;
}

// IPC Client Functions

export async function openPdf(path: string): Promise<OpenPdfResponse> {
  return invoke('open_pdf', { path });
}

export async function renderPage(page: number, width: number, height: number): Promise<RenderResponse> {
  return invoke('render_page', { page, width, height });
}

export async function renderThumbnail(page: number): Promise<RenderResponse> {
  return invoke('render_thumbnail', { page });
}

export async function navigatePage(page: number): Promise<NavigateResponse> {
  return invoke('navigate_page', { page });
}

export async function setZoom(percent: number): Promise<ZoomResponse> {
  return invoke('set_zoom', { percent });
}

export async function getPageDimensions(page: number): Promise<PageDimensionsResponse> {
  return invoke('get_page_dimensions', { page });
}
