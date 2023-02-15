<?php

namespace Jexactyl\Notifications;

use Jexactyl\Models\User;
use Jexactyl\Events\Event;
use Jexactyl\Models\Server;
use Illuminate\Bus\Queueable;
use Illuminate\Container\Container;
use Jexactyl\Events\Server\Installed;
use Illuminate\Notifications\Notification;
use Illuminate\Contracts\Queue\ShouldQueue;
use Jexactyl\Contracts\Core\ReceivesEvents;
use Illuminate\Contracts\Notifications\Dispatcher;
use Illuminate\Notifications\Messages\MailMessage;

class ServerInstalled extends Notification implements ShouldQueue, ReceivesEvents
{
    use Queueable;

    public Server $server;

    public User $user;

    /**
     * Handle a direct call to this notification from the server installed event. This is configured
     * in the event service provider.
     */
    public function handle(Event|Installed $event): void
    {
        $event->server->loadMissing('user');

        $this->server = $event->server;
        $this->user = $event->server->user;

        // Since we are calling this notification directly from an event listener we need to fire off the dispatcher
        // to send the email now. Don't use send() or you'll end up firing off two different events.
        Container::getInstance()->make(Dispatcher::class)->sendNow($this->user, $this);
    }

    /**
     * Get the notification's delivery channels.
     */
    public function via(): array
    {
        return ['mail'];
    }

    /**
     * Get the mail representation of the notification.
     */
    public function toMail(): MailMessage
    {
        return (new MailMessage())
            ->greeting('Olá ' . $this->user->username . '.')
            ->line('Seu servidor terminou de instalar e agora está pronto para você usar.')
            ->line('Nome do Servidor: ' . $this->server->name)
            ->action('Acesse e comece a usar', route('index'));
    }
}
