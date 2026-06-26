import { Injectable, NotFoundException } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository, In, FindOptionsWhere } from 'typeorm';
import { Notification, NotificationType } from './entities/notification.entity';
import { NotificationBroadcasterService } from '../websocket/notification-broadcaster.service';

@Injectable()
export class NotificationsService {
  constructor(
    @InjectRepository(Notification)
    private readonly notificationsRepository: Repository<Notification>,
    private readonly notificationBroadcaster: NotificationBroadcasterService,
  ) {}

  async create(
    userAddress: string,
    type: NotificationType | string,
    title: string,
    message: string,
    data?: Record<string, unknown>,
  ): Promise<Notification> {
    const notification = this.notificationsRepository.create({
      user_address: userAddress,
      type,
      title,
      message,
      data: data ?? null,
    });
    const saved = await this.notificationsRepository.save(notification);

    // Broadcast via WebSocket
    this.notificationBroadcaster.broadcastNewNotification(userAddress, {
      id: saved.id,
      type: saved.type,
      title: saved.title,
      message: saved.message,
      data: saved.data ?? undefined,
      created_at: saved.created_at,
    });

    return saved;
  }

  async findAllForUser(
    userAddress: string,
    page = 1,
    limit = 20,
    readFilter?: boolean,
    type?: string,
  ): Promise<{
    data: Notification[];
    total: number;
    page: number;
    limit: number;
    unreadCount: number;
  }> {
    const take = Math.min(limit, 100);
    const skip = (page - 1) * take;

    const where: Record<string, unknown> = { user_address: userAddress };
    if (readFilter !== undefined) {
      where.read = readFilter;
    }
    if (type) {
      where.type = type;
    }

    const [data, total] = await this.notificationsRepository.findAndCount({
      where: where as FindOptionsWhere<Notification>,
      order: { created_at: 'DESC' },
      skip,
      take,
    });

    // Get unread count
    const unreadCount = await this.notificationsRepository.count({
      where: { user_address: userAddress, read: false },
    });

    return { data, total, page, limit: take, unreadCount };
  }

  async markAsRead(id: number, userAddress: string): Promise<void> {
    const result = await this.notificationsRepository.update(
      { id, user_address: userAddress },
      { read: true },
    );

    if (!result.affected) {
      throw new NotFoundException('Notification not found');
    }

    // Broadcast read status via WebSocket
    this.notificationBroadcaster.broadcastNotificationRead(userAddress, id);
  }

  async markAllAsRead(userAddress: string): Promise<{ updated: number }> {
    const result = await this.notificationsRepository.update(
      { user_address: userAddress, read: false },
      { read: true },
    );

    return { updated: result.affected ?? 0 };
  }

  async markMultipleAsRead(
    userAddress: string,
    notificationIds: number[],
  ): Promise<{ updated: number }> {
    const result = await this.notificationsRepository.update(
      { user_address: userAddress, id: In(notificationIds) },
      { read: true },
    );

    return { updated: result.affected ?? 0 };
  }

  async remove(id: number, userAddress: string): Promise<void> {
    const notification = await this.notificationsRepository.findOne({
      where: { id, user_address: userAddress },
    });

    if (!notification) {
      throw new NotFoundException('Notification not found');
    }

    await this.notificationsRepository.softDelete(id);
  }

  async getUnreadCount(userAddress: string): Promise<number> {
    return this.notificationsRepository.count({
      where: { user_address: userAddress, read: false },
    });
  }
}
