/**
 * Gateway Profiles resource for Payment Orchestration Platform SDK
 */

import { AxiosInstance } from 'axios';
import {
  CreateGatewayProfileParams,
  UpdateGatewayProfileParams,
  GatewayProfile,
  GatewayProfileListParams,
  GatewayHealth,
} from '../types/gateway-profile';
import { PaginatedResponse } from '../types/common';

export class GatewayProfiles {
  private readonly httpClient: AxiosInstance;

  constructor(httpClient: AxiosInstance) {
    this.httpClient = httpClient;
  }

  /**
   * Create a new gateway profile
   */
  async create(params: CreateGatewayProfileParams): Promise<GatewayProfile> {
    const response = await this.httpClient.post<GatewayProfile>(
      '/v1/gateway_profiles',
      params
    );
    return response.data;
  }

  /**
   * Retrieve a gateway profile by ID
   */
  async retrieve(id: string): Promise<GatewayProfile> {
    const response = await this.httpClient.get<GatewayProfile>(
      `/v1/gateway_profiles/${id}`
    );
    return response.data;
  }

  /**
   * Update a gateway profile
   */
  async update(
    id: string,
    params: UpdateGatewayProfileParams
  ): Promise<GatewayProfile> {
    const response = await this.httpClient.patch<GatewayProfile>(
      `/v1/gateway_profiles/${id}`,
      params
    );
    return response.data;
  }

  /**
   * Delete a gateway profile
   */
  async delete(id: string): Promise<void> {
    await this.httpClient.delete(`/v1/gateway_profiles/${id}`);
  }

  /**
   * List gateway profiles
   */
  async list(
    params?: GatewayProfileListParams
  ): Promise<PaginatedResponse<GatewayProfile>> {
    const response = await this.httpClient.get<PaginatedResponse<GatewayProfile>>(
      '/v1/gateway_profiles',
      { params }
    );
    return response.data;
  }

  /**
   * Test gateway connection
   */
  async testConnection(id: string): Promise<{ success: boolean; latency_ms: number }> {
    const response = await this.httpClient.post<{ success: boolean; latency_ms: number }>(
      `/v1/gateway_profiles/${id}/test`
    );
    return response.data;
  }

  /**
   * Get gateway health status
   */
  async getHealth(id: string): Promise<GatewayHealth> {
    const response = await this.httpClient.get<GatewayHealth>(
      `/v1/gateway_profiles/${id}/health`
    );
    return response.data;
  }

  /**
   * Get all gateway health statuses
   */
  async getAllHealth(): Promise<GatewayHealth[]> {
    const response = await this.httpClient.get<GatewayHealth[]>(
      '/v1/gateway_profiles/health'
    );
    return response.data;
  }

  /**
   * Enable gateway profile
   */
  async enable(id: string): Promise<GatewayProfile> {
    return this.update(id, { status: 'active' });
  }

  /**
   * Disable gateway profile
   */
  async disable(id: string): Promise<GatewayProfile> {
    return this.update(id, { status: 'inactive' });
  }
}
