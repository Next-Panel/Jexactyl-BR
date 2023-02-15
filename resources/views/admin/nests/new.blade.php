@extends('layouts.admin')

@section('title')
    Novo Nest
@endsection

@section('content-header')
    <h1>Novo Nest<small>Configurar um novo nest para ser implantado em todos os nodes.</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Administrador</a></li>
        <li><a href="{{ route('admin.nests') }}">Nests</a></li>
        <li class="active">Novo</li>
    </ol>
@endsection

@section('content')
<form action="{{ route('admin.nests.new') }}" method="POST">
    <div class="row">
        <div class="col-md-12">
            <div class="box">
                <div class="box-header with-border">
                    <h3 class="box-title">Novo Nest</h3>
                </div>
                <div class="box-body">
                    <div class="form-group">
                        <label class="control-label">Nome</label>
                        <div>
                            <input type="text" name="name" class="form-control" value="{{ old('name') }}" />
                            <p class="text-muted"><small>Este deve ser um nome de categoria descritivo que englobe todos os eggs dentro do nest.</small></p>
                        </div>
                    </div>
                    <div class="form-group">
                        <label class="control-label">Descrição</label>
                        <div>
                            <textarea name="description" class="form-control" rows="6">{{ old('description') }}</textarea>
                        </div>
                    </div>
                    <div class="form-group">
                        <label class="control-label">Visibilidade do Nest</label>
                        <div>
                            <select name="private" class="form-control">
                                <option selected value="0">Público</option>
                                <option value="1">Privado</option>
                            </select>
                            <p class="text-muted"><small>Determina se os usuários podem implantar nesse nest.</small></p>
                        </div>
                    </div>
                </div>
                <div class="box-footer">
                    {!! csrf_field() !!}
                    <button type="submit" class="btn btn-primary pull-right">Salvar</button>
                </div>
            </div>
        </div>
    </div>
</form>
@endsection
