@extends('layouts.admin')

@section('title')
    Lista de Usu&aacute;rios
@endsection

@section('content-header')
    <h1>Usu&aacute;rios<small>Todos os usu&aacute;rios registrados no sistema.</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Administrador</a></li>
        <li class="active">Usu&aacute;rios</li>
    </ol>
@endsection

@section('content')
<div class="row">
    <div class="col-xs-12">
        <div class="box box-primary">
            <div class="box-header with-border">
                <h3 class="box-title">Lista de Usu&aacute;rios</h3>
                <div class="box-tools search01">
                    <form action="{{ route('admin.users') }}" method="GET">
                        <div class="input-group input-group-sm">
                            <input type="text" name="filter[email]" class="form-control pull-right" value="{{ request()->input('filter.email') }}" placeholder="Procurar">
                            <div class="input-group-btn">
                                <button type="submit" class="btn btn-default"><i class="fa fa-search"></i></button>
                                <a href="{{ route('admin.users.new') }}"><button type="button" class="btn btn-sm btn-primary" style="border-radius: 0 3px 3px 0;margin-left:-1px;">Criar Novo</button></a>
                            </div>
                        </div>
                    </form>
                </div>
            </div>
            <div class="box-body table-responsive no-padding">
                <table class="table table-hover">
                    <thead>
                        <tr>
                            <th>ID</th>
                            <th>Email</th>
                            <th>Nome do Cliente</th>
                            <th>Usu&aacute;rio</th>
                            <th class="text-center">Duas Etapas</th>
                            <th class="text-center">Approvados</th>
                            <th class="text-center"><span data-toggle="tooltip" data-placement="top" title="Servidores dos quais este usu&aacute;rio est&aacute; marcado como propriet&aacute;rio.">Servidores pr&oacute;prios</span></th>
                            <th class="text-center"><span data-toggle="tooltip" data-placement="top" title="Servidores que este usu&aacute;rio pode acessar porque est&atilde;o marcados como um subusu&aacute;rio.">Tem Accesso</span></th>
                            <th></th>
                        </tr>
                    </thead>
                    <tbody>
                        @foreach ($users as $user)
                            <tr class="align-middle">
                                <td><code>{{ $user->id }}</code></td>
                                <td><a href="{{ route('admin.users.view', $user->id) }}">{{ $user->email }}</a> @if($user->root_admin)<i class="fa fa-star text-yellow"></i>@endif</td>
                                <td>{{ $user->name_last }}, {{ $user->name_first }}</td>
                                <td>{{ $user->username }}</td>
                                <td class="text-center">
                                    @if($user->use_totp)
                                        <i class="fa fa-lock text-green"></i>
                                    @else
                                        <i class="fa fa-unlock text-red"></i>
                                    @endif
                                </td>
                                <td class="text-center">
                                    @if($user->approved)
                                        <i class="fa fa-check text-green"></i>
                                    @else
                                        <i class="fa fa-times text-red"></i>
                                    @endif
                                </td>
                                <td class="text-center">
                                    <a href="{{ route('admin.servers', ['filter[owner_id]' => $user->id]) }}">{{ $user->servers_count }}</a>
                                </td>
                                <td class="text-center">{{ $user->subuser_of_count }}</td>
                                <td class="text-center"><img src="https://www.gravatar.com/avatar/{{ md5(strtolower($user->email)) }}?s=100" style="height:20px;" class="img-circle" /></td>
                            </tr>
                        @endforeach
                    </tbody>
                </table>
            </div>
            @if($users->hasPages())
                <div class="box-footer with-border">
                    <div class="col-md-12 text-center">{!! $users->appends(['query' => Request::input('query')])->render() !!}</div>
                </div>
            @endif
        </div>
    </div>
</div>
@endsection
