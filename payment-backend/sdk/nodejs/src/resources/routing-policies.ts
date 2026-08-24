/**
 * Routing Policies resource for Payment Orchestration Platform SDK
 */

import { AxiosInstance } from 'axios';
import {
  CreateRoutingPolicyParams,
  UpdateRoutingPolicyParams,
  RoutingPolicy,
  RoutingPolicyListParams,
  SelectRouteParams,
  SelectedRoute,
} from '../types/routing-policy';
import { PaginatedResponse } from '../types/common';

export class RoutingPolicies {
  private readonly httpClient: AxiosInstance;

  constructor(httpClient: AxiosInstance) {
    this.httpClient = httpClient;
  }

  /**
   * Create a new routing policy
   */
  async create(params: CreateRoutingPolicyParams): Promise<RoutingPolicy> {
    const response = await this.httpClient.post<RoutingPolicy>(
      '/v1/routing_policies',
      params
    );
    return response.data;
  }

  /**
   * Retrieve a routing policy by ID
   */
  async retrieve(id: string): Promise<RoutingPolicy> {
    const response = await this.httpClient.get<RoutingPolicy>(
      `/v1/routing_policies/${id}`
    );
    return response.data;
  }

  /**
   * Update a routing policy
   */
  async update(
    id: string,
    params: UpdateRoutingPolicyParams
  ): Promise<RoutingPolicy> {
    const response = await this.httpClient.patch<RoutingPolicy>(
      `/v1/routing_policies/${id}`,
      params
    );
    return response.data;
  }

  /**
   * Delete a routing policy
   */
  async delete(id: string): Promise<void> {
    await this.httpClient.delete(`/v1/routing_policies/${id}`);
  }

  /**
   * List routing policies
   */
  async list(
    params?: RoutingPolicyListParams
  ): Promise<PaginatedResponse<RoutingPolicy>> {
    const response = await this.httpClient.get<PaginatedResponse<RoutingPolicy>>(
      '/v1/routing_policies',
      { params }
    );
    return response.data;
  }

  /**
   * Activate a routing policy
   */
  async activate(id: string): Promise<RoutingPolicy> {
    return this.update(id, { status: 'active' });
  }

  /**
   * Deactivate a routing policy
   */
  async deactivate(id: string): Promise<RoutingPolicy> {
    return this.update(id, { status: 'inactive' });
  }

  /**
   * Get active routing policy for operator
   */
  async getActive(): Promise<RoutingPolicy> {
    const response = await this.httpClient.get<RoutingPolicy>(
      '/v1/routing_policies/active'
    );
    return response.data;
  }

  /**
   * Select a route based on conditions
   */
  async selectRoute(params: SelectRouteParams): Promise<SelectedRoute> {
    const response = await this.httpClient.post<SelectedRoute>(
      '/v1/routing_policies/select',
      params
    );
    return response.data;
  }

  /**
   * Test routing policy
   */
  async testPolicy(
    id: string,
    params: SelectRouteParams
  ): Promise<SelectedRoute> {
    const response = await this.httpClient.post<SelectedRoute>(
      `/v1/routing_policies/${id}/test`,
      params
    );
    return response.data;
  }
}
