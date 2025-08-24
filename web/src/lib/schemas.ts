import { z } from 'zod';

// Base API response wrapper
export const ApiResponseSchema = z.object({
  success: z.boolean(),
  data: z.unknown(),
  error: z.string().nullable().optional(),
  message: z.string().nullable().optional(),
});

// Trending entry schema
export const TrendingEntrySchema = z.object({
  id: z.string().nullable().optional(),
  tmdb_id: z.number(),
  media_type: z.enum(['movie', 'tv']),
  title: z.string(),
  release_date: z.string().optional(),
  poster_path: z.string().nullable().optional(),
  backdrop_path: z.string().nullable().optional(),
  overview: z.string().optional(),
  source: z.string(),
  time_window: z.string(),
  rank: z.number(),
  score: z.number(),
  vote_average: z.number(),
  vote_count: z.number(),
  popularity: z.number().nullable().optional(),
  fetched_at: z.string(),
  expires_at: z.string(),
});

// Trending response schema
export const TrendingDataSchema = z.object({
  media_type: z.string(),
  time_window: z.string(),
  source: z.string(),
  entries: z.array(TrendingEntrySchema),
  total_results: z.number(),
  fetched_at: z.string(),
  expires_at: z.string(),
});

export const TrendingResponseSchema = ApiResponseSchema.extend({
  data: TrendingDataSchema,
});

// Movie schema
export const MovieSchema = z.object({
  id: z.string(),
  tmdb_id: z.number(),
  title: z.string(),
  year: z.number().nullable().optional(),
  overview: z.string().nullable().optional(),
  poster_path: z.string().nullable().optional(),
  backdrop_path: z.string().nullable().optional(),
  status: z.string(),
  monitored: z.boolean(),
  quality_profile_id: z.number().nullable().optional(),
  added: z.string(),
  updated: z.string(),
});

// Queue item schema
export const QueueItemSchema = z.object({
  id: z.string(),
  movieId: z.string(),
  movieTitle: z.string(),
  status: z.string(),
  progress: z.number(),
  size: z.number().optional(),
  indexer: z.string().optional(),
  downloadClient: z.string().optional(),
  added: z.string(),
  estimatedCompletionTime: z.string().nullable().optional(),
});

// Queue response schema
export const QueueResponseSchema = ApiResponseSchema.extend({
  data: z.object({
    records: z.array(QueueItemSchema),
    totalCount: z.number(),
  }),
});

// API response validation function
export function validateApiResponse<T>(
  data: unknown,
  schema: z.ZodSchema<T>,
  context: string
): T {
  try {
    return schema.parse(data);
  } catch (error) {
    console.error(`API response validation failed for ${context}:`, error);
    console.error('Received data:', data);
    
    if (error instanceof z.ZodError) {
      const issues = error.errors.map(err => 
        `${err.path.join('.')}: ${err.message}`
      ).join(', ');
      
      throw new Error(
        `API response validation failed for ${context}: ${issues}`
      );
    }
    
    throw new Error(
      `API response validation failed for ${context}: ${error}`
    );
  }
}

// Type exports for components
export type TrendingEntry = z.infer<typeof TrendingEntrySchema>;
export type TrendingData = z.infer<typeof TrendingDataSchema>;
export type TrendingResponse = z.infer<typeof TrendingResponseSchema>;
export type Movie = z.infer<typeof MovieSchema>;
export type QueueItem = z.infer<typeof QueueItemSchema>;
export type QueueResponse = z.infer<typeof QueueResponseSchema>;